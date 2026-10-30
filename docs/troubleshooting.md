# Solução de problemas

[← README](../README.md) · [Instalação](getting-started.md) · [Guia de uso](user-guide.md)

Execute comandos abaixo na raiz do projeto, onde ficam `package.json` e `src-tauri/`, salvo indicação diferente.

## `bun`, `node`, `cargo` ou `rustc`: command not found

Instale a ferramenta correspondente pelas instruções de [Primeiros passos](getting-started.md#2-preparar-as-ferramentas). Depois de instalar, abra outro terminal para carregar o PATH atualizado. Rust estável inclui Cargo.

## `package.json` não encontrado / erro ENOENT

O terminal está em outra pasta. Abra um terminal na pasta `harnessy`, confirme que ela contém `package.json` e execute novamente. Se o caminho tiver espaços, coloque-o entre aspas ao mudar de pasta.

## A primeira compilação está demorando

A primeira execução baixa e compila dependências Rust. Compilações posteriores reutilizam `src-tauri/target/` e tendem a ser mais rápidas. Mantenha conexão à internet e espaço livre para dependências/artefatos; o diretório de build pode ocupar muito mais espaço que o `.app` final.

Se houver erro, procure a primeira mensagem relevante antes do resumo final. Não é necessário apagar os lockfiles para tentar novamente.

## Erro de linker, SDK ou ferramentas Apple

Confira:

```bash
xcode-select -p
```

Se as ferramentas não estiverem instaladas, execute `xcode-select --install` e conclua o diálogo. Se o erro mencionar licença ou configuração pendente do Xcode, conclua a configuração indicada pela Apple. Reabra o terminal antes de recompilar.

## Rust antigo ou dependência incompatível

Confira `rustc --version`. Com instalação gerenciada por rustup, atualize o estável:

```bash
rustup update stable
```

Tente `bun run check:rust` novamente. O projeto foi validado com Rust 1.95; não há promessa de compatibilidade com todas as versões anteriores.

## Porta 1420 já está em uso

`bun run start` inicia seu próprio Vite. Se uma sessão de `bun run dev` ou outro `bun run start` estiver aberta, encerre-a com `Ctrl+C` no terminal correspondente.

Para descobrir quem está usando a porta:

```bash
lsof -nP -iTCP:1420 -sTCP:LISTEN
```

Identifique o processo antes de encerrá-lo. Não finalize processos desconhecidos ou troque apenas a porta do Vite: a URL configurada no Tauri também depende dela.

## O `.app` não existe

`bun run build` gera apenas os arquivos web em `dist/`. Para gerar o aplicativo desktop, use:

```bash
bun run desktop:build
```

Leia o caminho exibido ao final. O destino configurado atualmente é `.app`, não `.dmg`.

## O macOS bloqueou o aplicativo

O build atual é local e não possui assinatura/notarização de distribuição. Confirme que está abrindo o build que você gerou para sua arquitetura. Se decidir autorizar esse aplicativo local, use o procedimento do macOS em **Ajustes do Sistema → Privacidade e Segurança → Abrir Mesmo Assim**, quando essa opção estiver disponível.

Não é necessário desativar o Gatekeeper globalmente. Se o macOS informar arquitetura incompatível ou aplicativo danificado, não trate como simples aviso de assinatura: guarde a mensagem e gere novamente o build na máquina compatível.

## Fechei a janela, mas o aplicativo continua ativo

É o comportamento esperado. Use o menu **H → Abrir Harnessy** para voltar ou **H → Encerrar Harnessy** para sair. O H fica na barra superior do macOS.

Se estiver no modo de desenvolvimento, encerre também o comando ativo no terminal com `Ctrl+C`, caso permaneça executando.

## Abri o aplicativo, mas vejo uma versão antiga

Você pode estar usando uma cópia anterior em Applications. Encerre-a, gere o build e abra diretamente:

```bash
open src-tauri/target/release/bundle/macos/Harnessy.app
```

Substitua manualmente sua cópia em Applications. Não existe updater nesta versão.

## Uma conexão aparece como Indisponível, Sessão expirada ou Não verificado

- **Indisponível:** a CLI (`gh`, `codex` ou `claude`) não foi encontrada. O app procura no PATH do processo e em `~/.local/bin`, `~/.bun/bin`, `~/.npm-global/bin`, `~/.cargo/bin`, `/opt/homebrew/bin` e `/usr/local/bin`. Instale com o comando exibido no cartão ou crie um link simbólico em um desses diretórios.
- **Sessão expirada:** a CLI existe, mas não tem sessão válida. Execute `gh auth login --web`, `codex login` ou `claude auth login` no terminal e clique em **Verificar novamente**.
- **Não verificado:** a CLI não respondeu em 20 segundos ou retornou algo inesperado. Rode o comando de status manualmente (`gh auth status`, `codex login status`, `claude auth status`) para ver o erro completo.

Aliases de shell não afetam a verificação: o app executa o binário diretamente, sem passar pelo shell.

## O botão Iniciar está desabilitado

A execução de agentes ainda não foi implementada. Você pode usar navegação, tarefas locais, rascunhos e verificação de conexões, mas não executar agentes. Consulte o [estado atual](../README.md#o-que-está-disponível).

## O cabeçalho mostra “Prévia da interface”

É esperado quando você abre `http://127.0.0.1:1420` no navegador por `bun run dev`. Para ter o núcleo Rust, use `bun run start` e a janela desktop.

Se a própria janela desktop mostrar **Núcleo indisponível**, encerre e execute `bun run start` pelo terminal, copie o erro exibido e informe se ele ocorre também no build. Não há botão de reconexão implementado ainda.

## O rascunho desapareceu ou não está salvando

1. Confira se está usando o mesmo contexto: aplicativo desktop, prévia web e perfis de navegador não compartilham necessariamente os dados.
2. Se a interface informar erro de salvamento, copie o texto para um arquivo antes de fechar.
3. Confira se houve limpeza dos dados do navegador/WebView ou uso de perfil temporário.
4. Aguarde **Rascunho salvo neste dispositivo** depois de editar.

Não existe recuperação de histórico ou backup automático nesta etapa. Apagar o texto e aguardar o salvamento limpa o rascunho atual; não apague diretórios de dados do sistema para resolver problemas sem antes preservar o conteúdo.

## A tela está apertada ou cortada

A janela desktop tem mínimo de **1000 × 680**. Maximize a janela ou oculte o painel de contexto pelo botão à direita de Board/Chat. A área central tem rolagem; em telas menores, conteúdo abaixo da dobra deve ser alcançado por ela.

Se alguma ação ficar inacessível, registre tamanho da janela, escala da tela, tamanho de texto/zoom e uma captura de tela.

## Como relatar um problema útil

Inclua:

- Versão exibida em **Sobre esta fundação** e, se possível, revisão do código.
- macOS e arquitetura do Mac.
- Modo: desktop em desenvolvimento, build `.app` ou prévia web.
- Passos para reproduzir, resultado esperado e observado.
- Mensagem completa do erro relevante e captura sem informações privadas.

Para falhas de build/teste, inclua as saídas de `bun --version`, `node --version`, `rustc --version` e `cargo --version`. Remova segredos de logs antes de compartilhar.
