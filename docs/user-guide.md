# Guia de uso — fundação 0.1.0

[← README](../README.md) · [Instalação](getting-started.md) · [Solução de problemas](troubleshooting.md)

Esta versão permite explorar a experiência e preparar ideias. Ela ainda não envia prompts a modelos nem executa tarefas no seu código.

## Conheça a interface

### Barra lateral

- **Visão geral:** abre a área de trabalho, preservando a aba selecionada enquanto a janela está aberta.
- **Conexões:** mostra o estado planejado de GitHub e Codex.
- **Próxima tarefa / botão +:** abre o único rascunho de conversa disponível nesta etapa.
- **Sobre esta fundação:** informa versão, plataforma e capacidades implementadas.

“Meu espaço” ainda é uma identificação visual. Criar e trocar workspaces e manter várias conversas são funcionalidades futuras. O botão `+` não cria uma nova conversa nem apaga a atual.

### Board

Mostra as colunas planejadas e um estado vazio. Os zeros não são dados consultados do GitHub. **Configurar GitHub** leva à tela de conexões; não abre login nesta versão.

### Chat

1. Clique em **Preparar tarefa** ou em **Próxima tarefa**.
2. Descreva a ideia no campo **Próxima tarefa**.
3. Se quiser, clique em uma sugestão: ela acrescenta texto ao rascunho existente e devolve o foco ao campo.
4. Aguarde **Rascunho salvo neste dispositivo** antes de encerrar.

O botão **Iniciar** permanece desabilitado até a integração Codex. `Enter` insere uma nova linha; não há atalho de envio habilitado.

Um rascunho pode ter até **20.000 caracteres**. As sugestões também respeitam esse limite; se estiver próximo dele, edite o texto antes de inserir mais conteúdo.

### Painel de contexto

O botão à direita das abas mostra ou oculta o painel. Ele apresenta a estrutura planejada para repositórios e agentes; nenhum agente listado está ativo. Ocultar o painel amplia a área do board/chat sem apagar o rascunho.

### Conexões

GitHub aparece como **Não conectado** e Codex como **Não verificado**. Os botões estão desabilitados intencionalmente. Você não precisa fornecer tokens ou autenticar contas para explorar a fundação.

## Atalhos e teclado

| Ação | Atalho |
| --- | --- |
| Ir para Visão geral | `⌘1` |
| Ir para Conexões | `⌘2` |
| Percorrer controles | `Tab` / `Shift+Tab` |
| Alternar Board/Chat com foco nas abas | `←` / `→` |
| Primeira/última aba com foco nas abas | `Home` / `End` |
| Ativar botão com foco | `Enter` ou `Espaço` |
| Fechar o diálogo Sobre | `Esc` |

Em teclados sem teclas Home/End dedicadas, use as setas nas abas. Ao fechar **Sobre**, o foco volta ao botão que abriu o diálogo. Os atalhos de navegação não mudam a tela por trás do diálogo aberto.

O primeiro link acessível por teclado permite pular a barra lateral e ir ao conteúdo. O aplicativo também respeita a preferência **Reduzir movimento** do sistema.

## Rascunhos e dados locais

- Existe **um rascunho**, salvo no armazenamento local da WebView, com schema versionado.
- O salvamento ocorre após uma pequena pausa na digitação. A interface distingue **salvando**, **salvo** e **erro**.
- Trocar Board/Chat ou navegar para Conexões preserva o texto; recarregar usa a última gravação bem-sucedida.
- O aplicativo tenta gravar também ao sair da página, mas encerramento forçado ou falha do sistema pode perder as últimas alterações. Aguarde a confirmação de salvamento para reduzir esse risco.
- Prévia no navegador e aplicativo desktop têm armazenamentos distintos. Perfis/navegadores e instalações de desenvolvimento podem usar contextos diferentes; não conte com transferência automática entre eles.
- Não há sincronização, exportação ou backup automático. Para preservar algo importante, copie para um arquivo seu.
- Não há SQLite ou histórico de execuções implementado ainda.

Se o registro salvo estiver corrompido ou em schema incompatível, ele é preservado e o app avisa que o texto novo está apenas na memória. Ele só será substituído se você clicar em **Substituir registro anterior pelo texto atual**. Copie o registro antigo, caso precise investigá-lo, antes de confirmar. Se a gravação falhar, **Tentar salvar novamente** refaz a tentativa; navegar entre as telas não descarta a cópia atual em memória.

**Para limpar o rascunho:** selecione todo o texto no campo, apague e aguarde a confirmação de salvamento. Isso limpa o conteúdo atual; não há histórico de desfazer após reiniciar.

O rascunho não é armazenamento seguro para senhas, chaves de API ou conteúdo de `.env`. Nenhum prompt é enviado a uma LLM nesta versão; isso não significa que o armazenamento local seja criptografado ou apropriado para segredos.

## Fechar, reabrir e encerrar

| Ação | Resultado |
| --- | --- |
| Fechar a janela pelo botão vermelho | Oculta a janela e mantém o aplicativo ativo. |
| **H → Abrir Harnessy** na barra do macOS | Mostra a janela novamente. |
| Abrir o app pelo Dock/Finder | Solicita reabertura da janela. |
| **H → Encerrar Harnessy** | Encerra o aplicativo. |

O **H** fica na barra de menus do macOS, fora da janela. Ainda não existem agentes ou execuções para pausar/parar. O aplicativo não foi configurado para iniciar automaticamente com o login do macOS.

No modo de desenvolvimento, o terminal também controla o Vite/Tauri. Consulte [Primeiros passos](getting-started.md#4-abrir-no-modo-de-desenvolvimento) para encerrar a sessão inteira.

## Atualizar o aplicativo

Não existe atualização automática nesta etapa. Para atualizar pelo código:

1. Encerre a instância antiga.
2. Na raiz do projeto, execute `bun install --frozen-lockfile` e `bun run desktop:build`.
3. Abra o build recém-gerado.
4. Se usa uma cópia em Applications, substitua essa cópia pelo Finder.

Rodar um build não atualiza automaticamente a cópia em Applications. A versão pode permanecer `0.1.0` durante o desenvolvimento desta primeira entrega; consulte o [changelog](../CHANGELOG.md) para o conjunto de mudanças.
