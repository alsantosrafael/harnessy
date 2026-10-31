//! Ações: o que o usuário pode pedir sobre uma Tarefa. Cada Ação é um arquivo
//! TOML, embutido no app ou na pasta de dados do usuário, que declara papel,
//! acesso, modelo e o **pedido** final do prompt. O núcleo monta o início do
//! prompt, as ferramentas e a verificação; a Ação não escolhe nada disso.
//!
//! Este módulo lê, valida e preenche o pedido. Não conhece runtime nem a lista
//! de modelos: a validação de `modelo` fica em `flow` (regra 4).

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

/// Versão do formato de Ação aceita por este núcleo.
const SCHEMA: u32 = 1;
/// Pasta, dentro da pasta de dados do app, com as Ações do usuário.
const USER_DIR: &str = "actions";
/// Arquivo de Ação maior que isto é recusado antes de ser lido.
const MAX_FILE_BYTES: u64 = 16 * 1024;
const MAX_ID_BYTES: usize = 40;
/// Limite de cada variável preenchida no pedido (descrição e critérios).
const MAX_VARIABLE_BYTES: usize = 20_000;
/// Único papel com adaptador de execução nesta versão.
const PAPEL: &str = "implementador";
const SEM_ARQUIVOS: &str = "(nenhum encontrado)";
const SEM_CRITERIOS: &str = "(nenhum)";
const SEM_DESCRICAO: &str = "(sem descrição)";
const SEM_TITULO: &str = "(sem título)";

pub const IMPLEMENT: &str = "implementar";
pub const EXPLAIN: &str = "explicar";
pub const REVIEW: &str = "revisar";
pub const SPLIT: &str = "dividir-em-passos";
pub const REFINE: &str = "refinar";
const BUILTIN: [(&str, &str); 5] = [
    (IMPLEMENT, include_str!("embutidas/implementar.toml")),
    (EXPLAIN, include_str!("embutidas/explicar.toml")),
    (REVIEW, include_str!("embutidas/revisar.toml")),
    (SPLIT, include_str!("embutidas/dividir-em-passos.toml")),
    (REFINE, include_str!("embutidas/refinar.toml")),
];
/// Diff entregue a uma Ação sobre Run; o resto é cortado com aviso.
const MAX_DIFF_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Acesso {
    Leitura,
    Escrita,
}

/// Sobre o que a Ação age: a Tarefa (botão) ou o resultado de uma Run (automática).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Alvo {
    #[default]
    Tarefa,
    Run,
}

/// Forma da resposta, interpretada pelo núcleo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Saida {
    #[default]
    Texto,
    Revisao,
    Passos,
    Refinamento,
}

/// Quando uma Ação sobre Run roda: com sinal de risco, sempre, ou nunca sozinha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Quando {
    #[default]
    Risco,
    Sempre,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Origem {
    Embutida,
    Usuario,
    Repositorio,
}

/// Forma do arquivo TOML. `deny_unknown_fields` recusa campos fora do formato,
/// e informa a linha do campo recusado.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: u32,
    id: String,
    nome: String,
    descricao: String,
    papel: String,
    acesso: Acesso,
    modelo: Option<String>,
    esforco: Option<String>,
    max_turns: Option<u32>,
    contraprova: Option<bool>,
    exigir_prova: Option<bool>,
    alvo: Option<Alvo>,
    saida: Option<Saida>,
    quando: Option<Quando>,
    pedido: String,
}

/// Uma Ação validada. O pedido já está separado em texto fixo e variáveis.
#[derive(Debug, Clone)]
pub(crate) struct Action {
    pub(crate) id: String,
    pub(crate) nome: String,
    pub(crate) descricao: String,
    pub(crate) acesso: Acesso,
    /// Alias do modelo; a validação contra a lista do Claude Code fica em `flow`.
    pub(crate) modelo: Option<String>,
    /// Esforço de raciocínio; validado em `flow` contra o runtime.
    pub(crate) esforco: Option<String>,
    /// Teto de turnos desta Ação; o limite do núcleo é conferido em `flow`.
    pub(crate) max_turns: Option<u32>,
    /// Contraprova do teste (SPEC §5.2). Padrão: ligada; `false` pula a etapa.
    pub(crate) contraprova: bool,
    /// Sem teste que prove a mudança, a run ganha um ciclo de correção pedindo um.
    pub(crate) exigir_prova: bool,
    pub(crate) alvo: Alvo,
    pub(crate) saida: Saida,
    /// Só tem efeito em Ação sobre Run.
    pub(crate) quando: Quando,
    pedido: Vec<Parte>,
}

/// Valores das variáveis do pedido, preenchidos pelo núcleo a partir da Tarefa
/// (e, em Ação sobre Run, do resultado dela).
#[derive(Default)]
pub(crate) struct Contexto<'a> {
    pub(crate) tarefa: &'a str,
    pub(crate) descricao: &'a str,
    pub(crate) criterios: &'a str,
    /// Caminhos encontrados pelo núcleo, um por linha; vazio quando nenhum casou.
    pub(crate) arquivos_provaveis: &'a str,
    /// Diff da run contra a base (Ação sobre Run).
    pub(crate) diff: &'a str,
    /// Nome e resultado de cada check (Ação sobre Run).
    pub(crate) checks: &'a str,
    /// Veredito da contraprova em texto (Ação sobre Run).
    pub(crate) contraprova: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Parte {
    Texto(String),
    Variavel(Variavel),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variavel {
    Tarefa,
    Descricao,
    Criterios,
    ArquivosProvaveis,
    Diff,
    Checks,
    Contraprova,
}

impl Variavel {
    fn parse(nome: &str) -> Option<Self> {
        Some(match nome {
            "tarefa" => Self::Tarefa,
            "descricao" => Self::Descricao,
            "criterios" => Self::Criterios,
            "arquivos_provaveis" => Self::ArquivosProvaveis,
            "diff" => Self::Diff,
            "checks" => Self::Checks,
            "contraprova" => Self::Contraprova,
            _ => return None,
        })
    }

    /// Variáveis que só existem quando a Ação age sobre uma Run.
    fn da_run(self) -> bool {
        matches!(self, Self::Diff | Self::Checks | Self::Contraprova)
    }
}

/// Um Trecho do pedido: texto literal ou o nome entre `{{` e `}}`.
enum Trecho<'a> {
    Texto(&'a str),
    Nome(&'a str),
}

fn trechos(texto: &str) -> Vec<Trecho<'_>> {
    let mut saida = Vec::new();
    let mut resto = texto;
    while let Some(inicio) = resto.find("{{") {
        let Some(fim) = resto[inicio + 2..].find("}}") else {
            break;
        };
        saida.push(Trecho::Texto(&resto[..inicio]));
        saida.push(Trecho::Nome(&resto[inicio + 2..inicio + 2 + fim]));
        resto = &resto[inicio + 2 + fim + 2..];
    }
    saida.push(Trecho::Texto(resto));
    saida
}

/// Lê e valida uma Ação. Erros de formato citam a linha do arquivo.
pub(crate) fn parse(text: &str) -> Result<Action> {
    let raw: Raw = toml::from_str(text).map_err(|error| invalido(text, &error))?;
    if raw.schema != SCHEMA {
        return Err(AppError::Validation("Use schema = 1 nesta versão."));
    }
    if !id_valido(&raw.id) {
        return Err(AppError::Validation(
            "O id deve ter letras minúsculas, números e hífen, até 40 caracteres.",
        ));
    }
    if raw.nome.trim().is_empty() {
        return Err(AppError::Validation("O nome da Ação é obrigatório."));
    }
    if raw.papel != PAPEL {
        return Err(AppError::Validation(
            "Papel sem runtime nesta versão; use \"implementador\".",
        ));
    }
    if raw.acesso == Acesso::Leitura && raw.contraprova == Some(true) {
        return Err(AppError::Validation(
            "A contraprova só se aplica a Ação de escrita.",
        ));
    }
    if raw.modelo.as_deref().is_some_and(|m| m.trim().is_empty()) {
        return Err(AppError::Validation("O modelo não pode ser vazio."));
    }
    if raw.exigir_prova == Some(true)
        && (raw.acesso != Acesso::Escrita || raw.contraprova == Some(false))
    {
        return Err(AppError::Validation(
            "exigir_prova precisa de Ação de escrita com contraprova.",
        ));
    }
    if raw.max_turns == Some(0) {
        return Err(AppError::Validation("max_turns deve ser maior que zero."));
    }
    if raw.pedido.trim().is_empty() {
        return Err(AppError::Validation("O pedido da Ação é obrigatório."));
    }
    let alvo = raw.alvo.unwrap_or_default();
    let saida = raw.saida.unwrap_or_default();
    if saida != Saida::Texto && raw.acesso != Acesso::Leitura {
        return Err(AppError::Validation(
            "Saída estruturada (revisao, passos, refinamento) só vale para Ação de leitura.",
        ));
    }
    if alvo == Alvo::Run && raw.acesso != Acesso::Leitura {
        return Err(AppError::Validation(
            "Ação sobre Run só pode ler: ela opina, o núcleo decide.",
        ));
    }
    if raw.quando.is_some() && alvo != Alvo::Run {
        return Err(AppError::Validation(
            "\"quando\" só se aplica a Ação com alvo = \"run\".",
        ));
    }

    let mut pedido = Vec::new();
    for trecho in trechos(&raw.pedido) {
        match trecho {
            Trecho::Texto(texto) => pedido.push(Parte::Texto(texto.to_owned())),
            Trecho::Nome(nome) => match Variavel::parse(nome) {
                Some(variavel) if variavel.da_run() && alvo != Alvo::Run => {
                    return Err(AppError::Action(format!(
                        "linha {}: a variável \"{nome}\" só existe em Ação com alvo = \"run\".",
                        linha_da_variavel(text, nome)
                    )))
                }
                Some(variavel) => pedido.push(Parte::Variavel(variavel)),
                None => {
                    return Err(AppError::Action(format!(
                        "linha {}: variável desconhecida \"{nome}\". Use tarefa, descricao, criterios, arquivos_provaveis (ou diff, checks, contraprova em Ação sobre Run).",
                        linha_da_variavel(text, nome)
                    )))
                }
            },
        }
    }

    Ok(Action {
        id: raw.id,
        nome: raw.nome,
        descricao: raw.descricao,
        acesso: raw.acesso,
        modelo: raw.modelo,
        esforco: raw.esforco,
        max_turns: raw.max_turns,
        contraprova: raw.contraprova != Some(false) && raw.acesso == Acesso::Escrita,
        exigir_prova: raw.exigir_prova == Some(true),
        alvo,
        saida,
        quando: raw.quando.unwrap_or_default(),
        pedido,
    })
}

impl Action {
    /// Pedido com as variáveis preenchidas. É a parte final do prompt; o início vem do núcleo.
    pub(crate) fn render(&self, ctx: &Contexto<'_>) -> String {
        let mut saida = String::new();
        for parte in &self.pedido {
            match parte {
                Parte::Texto(texto) => saida.push_str(texto),
                Parte::Variavel(variavel) => saida.push_str(&valor(*variavel, ctx)),
            }
        }
        saida
    }
}

fn valor(variavel: Variavel, ctx: &Contexto<'_>) -> String {
    match variavel {
        Variavel::Tarefa => texto(ctx.tarefa, SEM_TITULO),
        Variavel::Descricao => texto(ctx.descricao, SEM_DESCRICAO),
        Variavel::Criterios => texto(ctx.criterios, SEM_CRITERIOS),
        Variavel::ArquivosProvaveis => texto(ctx.arquivos_provaveis, SEM_ARQUIVOS),
        Variavel::Diff => corte(ctx.diff, MAX_DIFF_BYTES, "(sem diff)"),
        Variavel::Checks => texto(ctx.checks, "(sem verificações)"),
        Variavel::Contraprova => texto(ctx.contraprova, "(não executada)"),
    }
}

/// Texto cortado num limite próprio (o diff tem orçamento maior que as demais).
fn corte(valor: &str, limite: usize, vazio: &str) -> String {
    let valor = valor.trim();
    if valor.is_empty() {
        return vazio.to_owned();
    }
    if valor.len() <= limite {
        return valor.to_owned();
    }
    let mut fim = limite;
    while !valor.is_char_boundary(fim) {
        fim -= 1;
    }
    format!("{}\n[diff truncado]", &valor[..fim])
}

/// Texto da variável, sem espaços nas pontas e com corte no limite.
fn texto(valor: &str, vazio: &str) -> String {
    let valor = valor.trim();
    if valor.is_empty() {
        return vazio.to_owned();
    }
    if valor.len() <= MAX_VARIABLE_BYTES {
        return valor.to_owned();
    }
    let mut fim = MAX_VARIABLE_BYTES;
    while !valor.is_char_boundary(fim) {
        fim -= 1;
    }
    format!("{}\n[texto truncado]", &valor[..fim])
}

fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_BYTES
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn invalido(text: &str, error: &toml::de::Error) -> AppError {
    let linha = error
        .span()
        .map_or(1, |span| linha_do_offset(text, span.start));
    let detalhe = error.message();
    let causa = if let Some(campo) = detalhe
        .strip_prefix("unknown field `")
        .and_then(|resto| resto.split('`').next())
    {
        format!("campo desconhecido \"{campo}\" nesta versão do formato")
    } else if let Some(campo) = detalhe
        .strip_prefix("missing field `")
        .and_then(|resto| resto.split('`').next())
    {
        format!("campo obrigatório \"{campo}\" ausente")
    } else {
        format!("TOML inválido ({detalhe})")
    };
    AppError::Action(format!("linha {linha}: {causa}"))
}

fn linha_do_offset(text: &str, offset: usize) -> usize {
    text.get(..offset).unwrap_or(text).matches('\n').count() + 1
}

/// Primeira linha em que a variável aparece, para o erro citar onde corrigir.
fn linha_da_variavel(text: &str, nome: &str) -> usize {
    let marca = format!("{{{{{nome}}}}}");
    text.lines()
        .position(|linha| linha.contains(&marca))
        .map_or(1, |indice| indice + 1)
}

/// Pasta das Ações do usuário, dentro da pasta de dados do app.
pub(crate) fn user_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(USER_DIR)
}

/// Uma entrada do catálogo: Ação válida ou o erro que a torna indisponível.
pub(crate) struct Entrada {
    pub(crate) id: String,
    pub(crate) origem: Origem,
    pub(crate) acao: Result<Action>,
    /// Ação do repositório ainda não confirmada: arquivo e hash a aprovar.
    pub(crate) pendente: Option<(String, String)>,
}

/// Forma da Ação na interface (`list_actions`). `erro` só existe quando inválida.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Resumo {
    id: String,
    nome: String,
    descricao: String,
    acesso: Option<Acesso>,
    /// `run`: roda sobre o resultado de uma Run, não aparece como botão da Tarefa.
    alvo: Option<Alvo>,
    /// `passos`: a resposta vira proposta de tarefas encadeadas.
    /// `refinamento`: a resposta vira perguntas e rascunho de descrição e critérios.
    saida: Option<Saida>,
    origem: Origem,
    /// Ação do repositório que precisa de confirmação local antes de rodar.
    pendente: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    erro: Option<String>,
}

impl Entrada {
    pub(crate) fn resumo(&self) -> Resumo {
        let pendente = self.pendente.is_some();
        match &self.acao {
            Ok(acao) => Resumo {
                id: acao.id.clone(),
                nome: acao.nome.clone(),
                descricao: acao.descricao.clone(),
                acesso: Some(acao.acesso),
                alvo: Some(acao.alvo),
                saida: Some(acao.saida),
                origem: self.origem,
                pendente,
                erro: None,
            },
            Err(error) => Resumo {
                id: self.id.clone(),
                nome: self.id.clone(),
                descricao: String::new(),
                acesso: None,
                alvo: None,
                saida: None,
                origem: self.origem,
                pendente,
                erro: Some(error.to_string()),
            },
        }
    }
}

/// Ações versionadas no repositório (`.harnessy/actions/`) e os hashes que o
/// usuário já confirmou neste computador (arquivo → sha256).
pub(crate) struct Repositorio<'a> {
    pub(crate) dir: PathBuf,
    pub(crate) aprovados: &'a HashMap<String, String>,
}

/// Pasta das Ações versionadas num repositório.
pub(crate) fn repo_dir(folder: &Path) -> PathBuf {
    folder.join(".harnessy").join(USER_DIR)
}

/// Catálogo: embutidas, do usuário e do repositório, nesta ordem de exibição.
/// Por id, repositório vence usuário, que vence embutida. Um arquivo inválido
/// aparece com o erro e também substitui, para o problema ficar visível. Ação do
/// repositório só roda depois que o hash do arquivo é confirmado (SPEC §15).
pub(crate) fn carregar(user_dir: &Path, repo: Option<&Repositorio<'_>>) -> Vec<Entrada> {
    let repositorio: Vec<Entrada> = repo
        .map(|repo| {
            ler_pasta(&repo.dir, Origem::Repositorio)
                .into_iter()
                .map(|(mut entrada, arquivo, hash)| {
                    let aprovado = repo.aprovados.get(&arquivo) == Some(&hash);
                    if entrada.acao.is_ok() && !aprovado {
                        entrada.acao = Err(AppError::Validation(
                            "Ação do repositório nova ou alterada: confirme antes de usar.",
                        ));
                        entrada.pendente = Some((arquivo, hash));
                    }
                    entrada
                })
                .collect()
        })
        .unwrap_or_default();
    let usuario: Vec<Entrada> = ler_pasta(user_dir, Origem::Usuario)
        .into_iter()
        .map(|(entrada, _, _)| entrada)
        .filter(|entrada| !repositorio.iter().any(|r| r.id == entrada.id))
        .collect();
    let mut entradas: Vec<Entrada> = BUILTIN
        .iter()
        .filter(|(id, _)| {
            !usuario.iter().any(|entrada| entrada.id == *id)
                && !repositorio.iter().any(|entrada| entrada.id == *id)
        })
        .map(|(id, text)| Entrada {
            id: (*id).to_owned(),
            origem: Origem::Embutida,
            acao: parse(text),
            pendente: None,
        })
        .collect();
    entradas.extend(usuario);
    entradas.extend(repositorio);
    entradas
}

/// Arquivos `.toml` de uma pasta, com nome do arquivo e sha256 do conteúdo.
fn ler_pasta(dir: &Path, origem: Origem) -> Vec<(Entrada, String, String)> {
    let Ok(lidos) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut arquivos: Vec<PathBuf> = lidos
        .filter_map(|entrada| entrada.ok().map(|entrada| entrada.path()))
        .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("toml") && path.is_file())
        .collect();
    arquivos.sort();

    let mut vistos = HashSet::new();
    arquivos
        .iter()
        .map(|path| {
            let (acao, hash) = match ler_texto(path) {
                Ok(text) => (parse(&text), sha256(&text)),
                Err(error) => (Err(error), String::new()),
            };
            let id = match &acao {
                Ok(acao) => acao.id.clone(),
                Err(_) => path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            };
            let acao = if vistos.insert(id.clone()) {
                acao
            } else {
                Err(AppError::Validation("Outra Ação já usa este id."))
            };
            let arquivo = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            (
                Entrada {
                    id,
                    origem,
                    acao,
                    pendente: None,
                },
                arquivo,
                hash,
            )
        })
        .collect()
}

fn ler_texto(path: &Path) -> Result<String> {
    let tamanho = fs::metadata(path).map_or(u64::MAX, |meta| meta.len());
    if tamanho > MAX_FILE_BYTES {
        return Err(AppError::Validation(
            "O arquivo da Ação passa de 16 KB e não foi lido.",
        ));
    }
    fs::read_to_string(path)
        .map_err(|_| AppError::Validation("Não foi possível ler o arquivo da Ação."))
}

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Hashes de Ações do repositório já confirmados neste workspace.
pub(crate) fn aprovacoes(db: &Connection, workspace_id: i64) -> Result<HashMap<String, String>> {
    let mut query =
        db.prepare("SELECT arquivo, sha256 FROM action_approvals WHERE workspace_id=?1")?;
    let rows = query.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Confirma o conteúdo atual (hash) de um arquivo de Ação do repositório.
pub(crate) fn aprovar(db: &Connection, workspace_id: i64, arquivo: &str, hash: &str) -> Result<()> {
    db.execute(
        "INSERT INTO action_approvals(workspace_id, arquivo, sha256, approved_at) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(workspace_id, arquivo) DO UPDATE SET sha256 = ?3, approved_at = ?4",
        params![workspace_id, arquivo, hash, crate::db::now()],
    )?;
    Ok(())
}

/// Ação nova criada pelo formulário da interface. Vira um arquivo TOML na pasta
/// do usuário; o arquivo é a fonte de verdade.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Nova {
    pub(crate) id: String,
    pub(crate) nome: String,
    pub(crate) descricao: String,
    pub(crate) acesso: Acesso,
    pub(crate) modelo: Option<String>,
    pub(crate) pedido: String,
}

#[derive(Serialize)]
struct Arquivo<'a> {
    schema: u32,
    id: &'a str,
    nome: &'a str,
    descricao: &'a str,
    papel: &'a str,
    acesso: Acesso,
    #[serde(skip_serializing_if = "Option::is_none")]
    modelo: Option<&'a str>,
    pedido: &'a str,
}

/// Texto TOML da Ação nova, já validado pelo mesmo `parse` dos arquivos.
pub(crate) fn texto_da_nova(nova: &Nova) -> Result<(String, Action)> {
    if BUILTIN.iter().any(|(id, _)| *id == nova.id) {
        return Err(AppError::Validation(
            "Este id é de uma Ação embutida; escolha outro.",
        ));
    }
    let arquivo = Arquivo {
        schema: SCHEMA,
        id: nova.id.trim(),
        nome: nova.nome.trim(),
        descricao: nova.descricao.trim(),
        papel: PAPEL,
        acesso: nova.acesso,
        modelo: nova.modelo.as_deref().filter(|m| !m.trim().is_empty()),
        pedido: &nova.pedido,
    };
    let text = toml::to_string(&arquivo)
        .map_err(|_| AppError::Validation("Não foi possível montar o arquivo da Ação."))?;
    let action = parse(&text)?;
    Ok((text, action))
}

/// Grava a Ação nova em `<pasta do usuário>/<id>.toml`. Não sobrescreve.
pub(crate) fn gravar_nova(user_dir: &Path, id: &str, text: &str) -> Result<PathBuf> {
    fs::create_dir_all(user_dir)
        .map_err(|_| AppError::Validation("Não foi possível criar a pasta de Ações."))?;
    let path = user_dir.join(format!("{id}.toml"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| AppError::Validation("Já existe uma Ação do usuário com este id."))?;
    std::io::Write::write_all(&mut file, text.as_bytes())
        .map_err(|_| AppError::Validation("Não foi possível gravar a Ação."))?;
    Ok(path)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::{fs, path::PathBuf};

    /// Pasta temporária de Ações do usuário, apagada ao sair de escopo.
    pub(crate) struct TempDir {
        pub(crate) path: PathBuf,
    }

    impl TempDir {
        pub(crate) fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "harnessy-test-actions-{label}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("pasta de teste");
            Self { path }
        }

        pub(crate) fn write(&self, name: &str, text: &str) {
            fs::write(self.path.join(name), text).expect("arquivo de teste");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TempDir;
    use super::*;

    const VALIDA: &str = "schema = 1\n\
        id = \"minha\"\n\
        nome = \"Minha\"\n\
        descricao = \"Teste.\"\n\
        papel = \"implementador\"\n\
        acesso = \"leitura\"\n\
        pedido = \"\"\"\n\
        Olhe {{tarefa}}.\n\
        \"\"\"\n";

    fn contexto() -> Contexto<'static> {
        Contexto {
            tarefa: "Título",
            descricao: "",
            criterios: "",
            ..Default::default()
        }
    }

    #[test]
    fn builtin_actions_parse_with_their_access() {
        let entradas = carregar(Path::new("/nao-existe-harnessy"), None);
        let ids: Vec<&str> = entradas.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "implementar",
                "explicar",
                "revisar",
                "dividir-em-passos",
                "refinar"
            ]
        );
        let implementar = entradas[0].acao.as_ref().unwrap();
        assert_eq!(implementar.acesso, Acesso::Escrita);
        assert!(entradas.iter().all(|e| e.origem == Origem::Embutida));
        assert_eq!(entradas[1].acao.as_ref().unwrap().acesso, Acesso::Leitura);
    }

    #[test]
    fn valid_action_fills_variables_and_empty_ones_get_placeholders() {
        let action = parse(VALIDA).unwrap();
        assert_eq!(action.render(&contexto()), "Olhe Título.\n");
        let vazio = Contexto {
            tarefa: "  ",
            descricao: "",
            criterios: "",
            ..Default::default()
        };
        let texto = parse(&VALIDA.replace("{{tarefa}}", "{{criterios}}")).unwrap();
        assert_eq!(texto.render(&vazio), "Olhe (nenhum).\n");
    }

    #[test]
    fn unknown_variable_is_rejected_with_its_line() {
        let text = "schema = 1\n\
            id = \"x\"\n\
            nome = \"X\"\n\
            descricao = \"d\"\n\
            papel = \"implementador\"\n\
            acesso = \"leitura\"\n\
            pedido = \"\"\"\n\
            Primeira linha.\n\
            Use {{dados}} aqui.\n\
            \"\"\"\n";
        let erro = parse(text).unwrap_err().to_string();
        assert!(erro.contains("linha 9"), "{erro}");
        assert!(erro.contains("dados"), "{erro}");
    }

    #[test]
    fn repository_actions_need_local_approval_of_their_exact_content() {
        let repo = TempDir::new("repo-acoes");
        let user = TempDir::new("repo-acoes-user");
        repo.write(
            "explicar.toml",
            &VALIDA.replace("\"minha\"", "\"explicar\""),
        );
        user.write(
            "explicar.toml",
            &VALIDA
                .replace("\"minha\"", "\"explicar\"")
                .replace("Minha", "Do usuário"),
        );

        let nenhum = HashMap::new();
        let pendente = Repositorio {
            dir: repo.path.clone(),
            aprovados: &nenhum,
        };
        let entradas = carregar(&user.path, Some(&pendente));
        let explicar: Vec<&Entrada> = entradas.iter().filter(|e| e.id == "explicar").collect();
        assert_eq!(explicar.len(), 1, "repositório vence usuário e embutida");
        assert_eq!(explicar[0].origem, Origem::Repositorio);
        assert!(explicar[0].acao.is_err());
        let (arquivo, hash) = explicar[0].pendente.clone().unwrap();
        assert_eq!(arquivo, "explicar.toml");
        assert!(explicar[0].resumo().pendente);

        let temp = crate::db::test_support::TempDatabase::new("acoes-aprovadas");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        aprovar(&db, 1, &arquivo, &hash).unwrap();
        let aprovados = aprovacoes(&db, 1).unwrap();
        let confirmado = Repositorio {
            dir: repo.path.clone(),
            aprovados: &aprovados,
        };
        let entradas = carregar(&user.path, Some(&confirmado));
        let explicar = entradas.iter().find(|e| e.id == "explicar").unwrap();
        assert!(explicar.acao.is_ok());
        assert!(explicar.pendente.is_none());

        repo.write(
            "explicar.toml",
            &VALIDA
                .replace("\"minha\"", "\"explicar\"")
                .replace("Olhe", "Veja"),
        );
        let entradas = carregar(&user.path, Some(&confirmado));
        let explicar = entradas.iter().find(|e| e.id == "explicar").unwrap();
        assert!(
            explicar.pendente.is_some(),
            "conteúdo alterado volta a pedir confirmação"
        );
    }

    #[test]
    fn new_action_from_the_form_is_validated_and_never_overwrites() {
        let user = TempDir::new("nova-acao");
        let nova = |id: &str, pedido: &str| Nova {
            id: id.into(),
            nome: "Adicionar testes".into(),
            descricao: "Cobre a tarefa com testes.".into(),
            acesso: Acesso::Escrita,
            modelo: Some("".into()),
            pedido: pedido.into(),
        };
        let (text, acao) = texto_da_nova(&nova("adicionar-testes", "Teste {{tarefa}}.")).unwrap();
        assert_eq!(acao.id, "adicionar-testes");
        assert!(
            !text.contains("modelo"),
            "modelo vazio não entra no arquivo"
        );
        gravar_nova(&user.path, &acao.id, &text).unwrap();
        assert!(gravar_nova(&user.path, &acao.id, &text).is_err());
        let entradas = carregar(&user.path, None);
        assert!(entradas
            .iter()
            .any(|e| e.id == "adicionar-testes" && e.acao.is_ok()));

        assert!(texto_da_nova(&nova("implementar", "x")).is_err());
        let erro = texto_da_nova(&nova("ok", "Use {{segredo}}."))
            .unwrap_err()
            .to_string();
        assert!(erro.contains("segredo"), "{erro}");
        assert!(texto_da_nova(&nova("ID Ruim", "x")).is_err());
    }

    #[test]
    fn unknown_field_is_rejected_including_planned_ones() {
        let text = format!("{VALIDA}ferramentas = [\"Bash\"]\n");
        let erro = parse(&text).unwrap_err().to_string();
        assert!(erro.contains("campo desconhecido"), "{erro}");
        // exigir_prova só com escrita e contraprova ligada.
        assert!(parse(&format!("{VALIDA}exigir_prova = true\n")).is_err());
        let escrita = VALIDA.replace("acesso = \"leitura\"", "acesso = \"escrita\"");
        assert!(
            parse(&format!("{escrita}exigir_prova = true\n"))
                .unwrap()
                .exigir_prova
        );
        assert!(parse(&format!(
            "{escrita}exigir_prova = true\ncontraprova = false\n"
        ))
        .is_err());
    }

    #[test]
    fn run_target_structured_output_and_schedule_follow_their_rules() {
        let sobre_run = VALIDA.replace(
            "acesso = \"leitura\"\n",
            "acesso = \"leitura\"\nalvo = \"run\"\nsaida = \"revisao\"\nquando = \"sempre\"\n",
        );
        let acao = parse(&sobre_run.replace("{{tarefa}}", "{{diff}}")).unwrap();
        assert_eq!(
            (acao.alvo, acao.saida, acao.quando),
            (Alvo::Run, Saida::Revisao, Quando::Sempre)
        );
        let ctx = Contexto {
            diff: "+nova linha",
            ..Default::default()
        };
        assert_eq!(acao.render(&ctx), "Olhe +nova linha.\n");

        // Variável de Run fora de Ação sobre Run.
        let erro = parse(&VALIDA.replace("{{tarefa}}", "{{diff}}"))
            .unwrap_err()
            .to_string();
        assert!(erro.contains("alvo = \"run\""), "{erro}");
        // `quando` sem alvo run; saída estruturada em escrita; alvo run em escrita.
        assert!(parse(&format!("{VALIDA}quando = \"sempre\"\n")).is_err());
        let escrita = VALIDA.replace("acesso = \"leitura\"", "acesso = \"escrita\"");
        assert!(parse(&format!("{escrita}saida = \"passos\"\n")).is_err());
        assert!(parse(&format!("{escrita}alvo = \"run\"\n")).is_err());
    }

    #[test]
    fn invalid_values_are_rejected() {
        let sem_schema = VALIDA.replace("schema = 1", "schema = 2");
        assert!(parse(&sem_schema).is_err());
        let id_ruim = VALIDA.replace("id = \"minha\"", "id = \"Minha Acao\"");
        assert!(parse(&id_ruim).is_err());
        let papel = VALIDA.replace("implementador", "tester");
        assert!(parse(&papel).is_err());
        let contraprova = format!("{VALIDA}contraprova = true\n");
        let erro = parse(&contraprova).unwrap_err().to_string();
        assert!(erro.contains("contraprova"), "{erro}");
    }

    #[test]
    fn user_action_overrides_builtin_and_broken_files_are_listed() {
        let temp = TempDir::new("override");
        temp.write("implementar.toml", &VALIDA.replace("minha", "implementar"));
        temp.write("quebrada.toml", "schema = 1\nid = \"quebrada\"\n");
        temp.write("notas.txt", "ignorado");

        let entradas = carregar(&temp.path, None);
        let ids: Vec<&str> = entradas.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "explicar",
                "revisar",
                "dividir-em-passos",
                "refinar",
                "implementar",
                "quebrada"
            ]
        );

        let implementar = entradas.iter().find(|e| e.id == "implementar").unwrap();
        assert_eq!(implementar.origem, Origem::Usuario);
        assert_eq!(implementar.resumo().nome, "Minha");

        let quebrada = entradas.iter().find(|e| e.id == "quebrada").unwrap();
        let resumo = quebrada.resumo();
        assert!(resumo
            .erro
            .as_deref()
            .unwrap_or("")
            .contains("campo obrigatório"));
        assert_eq!(resumo.acesso, None);
    }

    #[test]
    fn duplicate_user_ids_keep_the_first_file_only() {
        let temp = TempDir::new("duplicate");
        temp.write("a.toml", &VALIDA.replace("minha", "dup"));
        temp.write("b.toml", &VALIDA.replace("minha", "dup"));
        let entradas = carregar(&temp.path, None);
        let dup: Vec<&Entrada> = entradas.iter().filter(|e| e.id == "dup").collect();
        assert_eq!(dup.len(), 2);
        assert!(dup[0].acao.is_ok());
        assert!(dup[1].acao.is_err());
    }
}
