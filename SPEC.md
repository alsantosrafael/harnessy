# Harnessy — especificação consolidada

Versão do documento: 1.1 · Atualizado em 2026-10-29  
Versão inicial do produto: **0.1.0**  
Status: decisões de produto aprovadas; implementação incremental  
Plataforma inicial: macOS, uso individual, local-first  
Stack alvo: **Tauri 2 + Rust + React/TypeScript + SQLite**; frontend com **Bun + Vite + Vitest**.

## 1. Objetivo

Construir um aplicativo desktop leve para comandar agentes de desenvolvimento, organizar tarefas no GitHub Projects e acompanhar entregas com revisão e validação funcional independentes.

O usuário pode pedir: “pegue o card X”, “crie uma tarefa no board Y”, “implemente e abra PR”, “revise este PR” ou “valide esta aplicação”. A interface reúne board, conversas, progresso, alterações e evidências.

Inicialmente todos os papéis usam **Codex**, em sessões independentes e aproveitando autenticação suportada do usuário. O Harnessy coordena o agente existente; não implementa um loop próprio de inferência no MVP. Claude Code e providers via API são extensões futuras, com contratos próprios.

O aplicativo deve poder ser distribuído para outras pessoas futuramente, cada uma com seus dados e credenciais locais. Windows e Linux são alvos futuros; não se presume portabilidade de sandbox, autenticação e instalador sem validação por sistema.

## 2. Princípios e base de evidências

Esta especificação consolida as decisões da entrevista e recomendações pesquisadas em fontes primárias. O relatório [Práticas de harness](docs/research/harness-practices.md) contém referências, limitações e a distinção entre evidência e recomendação derivada.

Princípios adotados:

1. **Usabilidade acima da exposição de mecanismos internos:** intenção, responsável, progresso e próxima ação devem ser evidentes.
2. **Contexto de alto sinal:** recuperar informação sob demanda em vez de enviar históricos e repositórios inteiros [A2].
3. **Progresso incremental e verificável:** checkpoints, critérios de aceite e estado real são mais importantes que uma mensagem de “concluído” [A1].
4. **Repositório navegável:** instruções curtas como índice, documentação localizada e regras arquiteturais verificáveis [O1].
5. **Avaliar o sistema completo:** modelo, contexto, ferramentas, políticas e ambiente; resultado funcional não equivale a nota de prontidão estática [A3].
6. **Autoridade no núcleo:** UI e conteúdo dos agentes não ampliam permissões; isolamento técnico e aprovação são mecanismos diferentes [A4, O4].
7. **Eficiência com qualidade:** otimizar consumo por entrega validada, incluindo tentativas malsucedidas, não somente tokens por chamada.

Os artigos não prescrevem esta arquitetura de produto. Políticas de merge, limites e papéis abaixo são decisões do Harnessy.

## 3. Escopo

### Primeira versão funcional

- Aplicativo desktop macOS com janela própria, board, chat e painel contextual.
- Conexão visual GitHub e Codex; seleção de repositórios e GitHub Projects.
- Workspaces com um ou vários repositórios; múltiplas conversas por workspace.
- Consultar, criar, editar e mover cards; GitHub como fonte oficial.
- Fluxos independentes de refinamento, implementação, revisão e teste, além do fluxo composto.
- Worktrees por tarefa e por subagente escritor paralelo; delegação com limites.
- Branch, commit, push, PR, revisão/comentário e merge explícito pelo usuário.
- Testes locais, deploy sob demanda por workflow autorizado e Playwright contra ambiente remoto.
- Persistência, retomada explícita, cancelamento, checkpoints e reconciliação de efeitos.
- Configurações e definições de agentes/workflows versionadas; SemVer desde o início.
- Políticas de contexto, observabilidade e avaliação do consumo disponível.
- Isolamento verificado que bloqueie credenciais locais inclusive pelo shell e subprocessos.

### Futuro

- Claude Code, agentes próprios via APIs, roteamento entre providers.
- Builds Windows/Linux, distribuição assinada e atualização verificável.
- Preview remoto exclusivo por tarefa como opção, não infraestrutura automática obrigatória do MVP.
- Scanner estático de prontidão do repositório, após validar ferramenta e cobertura.

### Fora do escopo inicial

- SaaS, contas centrais, colaboração multiusuário e agendamento distribuído.
- Execução disparada automaticamente por movimentação de card ou webhook.
- Merge ou deploy em produção sem ação explícita do usuário.
- Alteração do executável instalado pelo próprio agente em execução.
- Garantia de custo financeiro exato quando a integração não fornece dados suficientes.

## 4. Modelo de domínio

| Conceito | Definição |
| --- | --- |
| Workspace | Projeto local com repositórios, conexão a Projects e políticas/configurações. |
| Repositório | Identidade GitHub, clone gerenciado, branch base e instruções aplicáveis. |
| Card | Item do GitHub Projects, vinculado a issue quando houver; não equivale sempre a issue. |
| Conversa | Histórico de intenção e decisões; pode conter várias execuções e vínculos. |
| Tarefa | Objetivo e critérios de aceite confirmados, vinculados a cards/PRs. |
| Execução | Tentativa orquestrada com configuração congelada, limites e evidências. |
| Sessão de agente | Papel, runtime/modelo efetivos, identificadores e histórico próprios. |
| Worktree | Checkout/branch isolados para escrita; não é sandbox de segurança. |
| Handoff | Estado estruturado para outro papel ou sucessor continuar o trabalho. |
| Check | Verificação reproduzível com versão, comando, resultado e artefatos. |
| Publicação | Efeito externo reconciliável: card, comentário, commit, PR, review ou deploy. |
| Reserva de ambiente | Exclusividade para deploy e validação de uma versão em ambiente compartilhado. |

Vínculos entre vários repositórios e PRs pertencem à mesma tarefa. Não existe atomicidade de merge entre repositórios: a UI explicita resultados parciais e dependências.

## 5. Experiência e design system

### Estrutura

- Barra lateral: workspaces, conversas e conexões.
- Centro do workspace: abas **Board** e **Chat**.
- Painel contextual recolhível: card, PR, execução, diff e evidências, sem perder o contexto central.
- Várias conversas por projeto; cards, PRs e repositórios podem ser anexados à conversa.
- Cada execução mostra papel responsável, versão em análise, etapa, fila, bloqueios e ações disponíveis.

Board suporta criação/edição e mudança de status com confirmação de sincronização. Falha de escrita preserva o rascunho, informa o erro e permite retry reconciliado. Alterações externas são reconciliadas sem sobrescrita silenciosa de edições concorrentes.

Chat aceita prompts e instruções durante a execução. Distingue instrução enviada, entregue e incorporada quando observável; não simula suporte ausente no adaptador. Tarefas começam por prompt ou botão **Iniciar tarefa**, nunca apenas ao mover um card.

### Direção visual

Inspiração: clareza do Notion, personalidade contida do PostHog e familiaridade do GitHub. Layout content-first, superfícies neutras, bordas discretas, tipografia legível, poucos acentos e ícones vetoriais consistentes. Sem copiar marcas ou transformar o aplicativo em painel ornamental.

- Tokens primitivos, semânticos e de componentes para cores, tipografia, espaçamento e estados.
- Bun gerencia dependências/scripts frontend com `bun.lock`; Vite é o bundler/dev server e Vitest executa regressões por `bun run test`. Não usar o runner nativo `bun test` como substituto implícito.
- Escala de espaçamento consistente; hierarquia visual clara e densidade ajustada ao trabalho desktop.
- Tema claro inicial; arquitetura de tokens preparada para tema escuro, que só é anunciado após validação independente.
- Contraste de texto normal >= 4,5:1; controles e foco distinguíveis; estados não dependem apenas de cor.
- Teclado, foco visível, nomes acessíveis, alternativas a arrastar e respeito a movimento reduzido.
- Estados vazios, loading, desconexão, erro e recuperação desenhados como partes do produto.
- Conteúdo gerado não executa HTML/scripts; links externos passam por validação.

### Interações e componentes de referência

O design system documenta anatomia, variantes, tokens, teclado e estados `default`, `hover`, `pressed`, `focus`, `disabled`, `loading` e `error` quando aplicáveis. Uma galeria local de componentes permite validar consistência sem executar agentes.

| Elemento | Contrato de interação |
| --- | --- |
| RunControl | Exibe pausa/parada solicitada e confirmada; impede ação incompatível, com motivo acessível. |
| TaskCard | Status, responsável e vínculo; menu permite mover sem drag-and-drop. |
| ChatComposer | Preserva rascunho, distingue envio de recebimento, não submete ao usar composição de texto IME; atalho de envio documentado. |
| ApprovalDialog | Ação, escopo, destino e consequência; confirmação vinculada ao pedido; foco contido e devolvido ao acionador ao fechar. |
| EvidencePanel | Versão, resultado, artefatos e origem; carrega conteúdo pesado sob demanda. |
| ConnectionStatus | Conectado, verificando, expirado, sem permissão e indisponível, com ação específica. |

Painéis não modais não prendem foco. Ao abrir um detalhe, preservar seleção, scroll e rascunho da origem. Novas mensagens não forçam scroll se o usuário estiver lendo acima; mostrar indicador de novidades. Notificações de progresso para leitores de tela são agrupadas, sem anunciar cada token.

Paleta, tamanho e espaçamento vivem em três camadas: primitivos → aliases semânticos → tokens de componente. Valores de cor não são repetidos em componentes. Alteração de tema ocorre nos aliases, mantendo estados e contraste. Desabilitado não pode ser a única explicação de por que uma ação está indisponível.

Comportamentos de foco seguem WCAG 2.2 AA como base aplicável à interface web; buscar foco totalmente visível como meta adicional, sem apresentar critério AAA como obrigação AA.

### Medição de desempenho

Metas iniciais, medidas em build de produção no Mac de referência documentado:

| Métrica | Meta |
| --- | --- |
| Feedback visual de ação | <= 100 ms, p95 |
| Troca de tela com estado carregado | <= 150 ms, p95 |
| Inicialização com estado local | <= 2 s, p95 em cenário documentado |
| Rolagem/animação | Buscar 60 fps, sem travas perceptíveis sob streaming |
| Ociosidade | Sem polling agressivo; CPU próxima de zero |

Registrar memória residente e crescimento ao longo do tempo; separar UI/núcleo de Codex, containers e navegadores. Definir orçamento de memória após baseline real, sem prometer número não medido.

Históricos/listas virtualizados, paginação, deltas em lotes, buffers limitados, artefatos sob demanda e tarefas pesadas fora da WebView. Fixture inicial: board com 1.000 cards, conversa com 10.000 mensagens e três streams concorrentes; crescimento dos buffers deve permanecer limitado.

Aplicar regras React pertinentes à WebView: assinaturas de estado por item/seleção, listeners IPC compartilhados com cleanup, carregamento condicional de diff/trace, operações independentes em paralelo e atualizações não urgentes separadas da digitação. Memoização só onde profiling justificar; transições não substituem redução de trabalho e virtualização. Não importar infraestrutura Next.js/SSR sem necessidade.

Medições usam cenário, hardware, versão do SO/WebView e tamanho dos artefatos registrados. Separar latência de feedback da conclusão de rede. Perfis de CPU/memória e regressões devem acompanhar mudanças no caminho de streaming, não apenas benchmarks sintéticos isolados.

## 6. Arquitetura

```text
React / TypeScript — WebView
        │ comandos e eventos IPC tipados e restritos
Tauri / Rust — núcleo local confiável
 ├─ Workspaces, conversas e configuração
 ├─ Orquestrador e políticas de workflow
 ├─ Supervisor de processos e fila
 ├─ Contexto, handoffs e orçamento
 ├─ SQLite e artefatos locais
 ├─ Adaptador Codex app-server — stdio
 ├─ Git/worktrees e executor isolado
 ├─ GitHub / OAuth / armazenamento seguro
 └─ CI, deploy, reserva de ambiente e evidências
```

Rust concentra validação, permissões, processo de fundo e efeitos externos. React projeta estado e solicita ações específicas; não recebe credenciais nem um comando IPC genérico de shell.

Interfaces por capacidade evitam fingir equivalência entre runtimes. Cada adaptador informa suporte real a autenticação, retomada, interrupção, instruções durante turno, uso e limites. Funcionalidade ausente aparece como indisponível.

### Codex primeiro

Preferir **Codex app-server via stdio**, usando protocolo e schemas correspondentes à versão suportada [O2]. Modelar thread, turn e item; distinguir delta, término, aprovação e interrupção. Inicialização, requests bidirecionais e saída inesperada são tratados pelo supervisor.

- Pin/teste da versão do runtime e registro da versão efetiva.
- Correlacionar IDs; drenar stdout do protocolo e stderr separadamente, com limites de retenção.
- Não expor servidor de controle à rede por conveniência.
- Não reenviar efeitos após crash sem reconciliação.
- Turno interrompido não implica rollback de arquivos.
- SDK de agente não deve ser importado no renderer como biblioteca de navegador.

Prova técnica obrigatória antes da integração de produção: autenticação existente, isolamento do executor e descendentes, eventos, aprovações, cancelamento e retomada. Autenticação disponível não é autorização para acesso ao diretório pessoal. Se login e isolamento exigidos não forem conciliáveis na versão testada, a execução fica bloqueada até decisão explícita de arquitetura.

### Módulos profundos e interfaces

Cada módulo esconde complexidade operacional atrás de uma interface pequena que inclui invariantes, erros, ordenação e cancelamento, não apenas tipos. Chamadores não coordenam detalhes de retries, locks e persistência que pertencem à implementação do módulo.

| Módulo | Interface conceitual | Complexidade escondida |
| --- | --- | --- |
| Execuções | Iniciar, consultar/observar e solicitar controle | Transições, fila, checkpoints, limites e correlação. |
| Sessões Codex | Descobrir capacidades, iniciar/retomar, enviar e interromper | Protocolo, processos, autenticação suportada e reconexão. |
| Publicações | Solicitar efeito autorizado e consultar resultado | Intenção durável, idempotência e reconciliação. |
| Ambientes | Reservar, publicar versão e validar disponibilidade | Exclusividade, jobs externos, expiração e versões parciais. |
| Configuração | Resolver snapshot validado para uma execução | Precedência, confiança, compatibilidade e referências de segredos. |

As interfaces são orientações, não obrigação de criar uma classe por linha. Evitar camadas de mero repasse e abstrações genéricas para providers ainda não implementados. O seam de adaptador Codex existe para separar protocolo externo do domínio e permitir fixtures de contrato; um futuro provider não precisa fingir equivalência funcional.

Dependências com efeitos são recebidas explicitamente. Relógio, processos e integrações podem variar nos testes sem expor detalhes internos aos chamadores. Falhas tipadas incluem categoria, possibilidade de retry, efeitos confirmados e ação de resolução; mensagens livres não decidem política.

## 7. Agentes e contratos

Definições próprias inspiradas nas skills locais, adaptadas ao orquestrador. Não copiar instruções de commit, revisão ou delegação que conflitem com as políticas centrais.

| Papel | Inspiração | Entrega |
| --- | --- | --- |
| Refinador | grill-me / grilling | Objetivo, escopo, critérios e dúvidas resolvidas. |
| Implementador | implement, tdd, diagnosing-bugs | Alterações, checks, resumo e pendências. |
| Revisor | code-review | Aderência à tarefa, padrões, correção e achados fundamentados. |
| Tester | diagnosing-bugs e Playwright | Cenários funcionais, esperado/observado, evidências e bugs. |

Refinador consulta primeiro card, documentação e código; pergunta uma questão por vez somente sobre decisões não resolvidas. Critérios propostos precisam de confirmação antes de iniciar implementação ambígua.

Todos usam Codex inicialmente, com sessão independente. Não repassar conversa privada do implementador ao revisor; compartilhar tarefa, critérios, código, diff, checks e evidências. Tester recebe contexto funcional, ambiente e versão, preservando sua independência para explorar além do caminho feliz.

### Contratos estruturados

Entregas incluem `run_id`, papel, versão do contrato, estado, referências de código e artefatos. Validar schema e evidências; texto livre não autoriza transição.

Toda definição de agente declara: finalidade, entradas mínimas, capacidades permitidas, condições de parada, saída estruturada, tratamento de incerteza e handoff. Skills são carregadas por necessidade da tarefa, não todas em todo prompt. Uma skill instrucional não concede ferramenta ou permissão. Alterações nas definições seguem revisão, versionamento e evals antes de virar padrão.

- Implementação: resumo, arquivos, checks, limitações e SHAs candidatos.
- Revisão: SHAs analisados, critérios avaliados, achados e `approve | request_changes | inconclusive`.
- Achado: severidade, título, problema/impacto/evidência, caminho/linha quando aplicável.
- Teste: versão publicada, critérios/cenários executados, resultado, evidências e bugs relacionados.
- Handoff: objetivo, critérios, decisões, restrições, versão, estado atual, reprodução, verificações e próxima ação; referências às fontes originais.

Dados essenciais de estado pertencem ao núcleo, não apenas à mensagem final do modelo.

## 8. Workflow principal

1. Usuário inicia tarefa; resolver objetivo, critérios, repositórios e configuração.
2. Registrar execução e SHAs base imutáveis; preparar worktrees e ambiente filtrado.
3. Executar preparação e baseline: distinguir falha preexistente de regressão.
4. Implementador trabalha incrementalmente e executa checks relevantes.
5. Orquestrador verifica diff e checks finais; registra versão candidata.
6. Publicar branch/PR de forma reconciliável; checks reprovados permitem apenas draft claramente identificado.
7. Revisor independente examina exatamente os SHAs candidatos. Correções voltam ao implementador; resultado inconclusivo bloqueia avanço.
8. Com revisão e checks obrigatórios aprovados, adquirir reserva e disparar deploy autorizado sob demanda.
9. Confirmar versão efetivamente publicada e health checks; então entregar ao tester.
10. Tester valida critérios e fluxos com Playwright.
11. Bug gera card vinculado à tarefa/PR com contexto funcional, passos, esperado/observado, versão e evidências; encaminhar ao implementador original.
12. Se a sessão não puder continuar, gerar handoff para sucessor. Não presumir percentual de janela de contexto que o runtime não exponha.
13. Correção passa por checks, revisão, deploy e retorno ao tester original ou sucessor com handoff persistido.
14. Critérios e verificações aprovados: mover para status mapeado como **Pronto para merge** e apresentar entrega ao usuário.
15. Usuário revisa na UI ou no GitHub e decide o merge.

Até **três ciclos de correção** por padrão; ao atingir, pedir orientação. Definir ciclo como nova rodada de alterações submetida ao gate que havia falhado, evitando contabilização diferente entre agentes. Falha de infraestrutura não é bug funcional nem aprovação; retries de infraestrutura têm limite próprio.

Qualquer mudança de SHA invalida aprovações e validações afetadas. Antes de review, deploy e merge, confrontar versões. Configurar dependências entre PRs de vários repositórios e explicitar entregas parciais.

Sem alterações: finalizar `no_changes`, sem criar PR vazio. Fluxos independentes podem terminar sem passar por etapas não solicitadas.

## 9. Workspaces, subagentes e concorrência

- Workspace reúne repositórios e configuração; cada tarefa escritora recebe worktree/branch própria.
- Subagentes escritores paralelos recebem worktrees próprias; principal integra e verifica conjunto, sem compartilhar diretório mutável.
- Subagentes de pesquisa/revisão acessam apenas contexto e operações necessários.
- Uma implementação ativa por workspace e **três sessões Codex ativas no aplicativo** por padrão, incluindo subagentes.
- Fila visível, reordenável e cancelável; sessões aguardando humano/etapa não mantêm chamadas desnecessárias.
- Não criar deadlock: agente principal aguardando filho não pode reter todos os slots necessários aos filhos.
- Separar portas, bancos locais, dados de teste e artefatos entre execuções.
- Limites configuráveis por workspace; limite global é autoridade superior.

## 10. Pausa, cancelamento e continuidade

Botões visuais por conversa e execução:

- **Pausar:** solicitar checkpoint e impedir novas etapas; aguardar ponto seguro. Exibir `pausing` até confirmação. Não prometer suspensão instantânea de operação externa.
- **Retomar:** verificar estado/checkpoint e continuar.
- **Parar:** cancelar execução e processos controlados, preservando efeitos externos e evidências. Exibir operações externas que não puderam ser interrompidas.

Fechar a janela mantém trabalho local em segundo plano. **Encerrar Harnessy** é ação explícita com tratamento das execuções ativas. Processo supervisor não depende da renderização da janela.

Após restart: recuperar conversas, fila, worktrees e checkpoints; marcar trabalho abandonado como interrompido e oferecer retomada humana. Reconciliar GitHub, CI e ambiente antes de repetir ações. Se thread original não for recuperável, criar sucessor com handoff.

Checkpoint mínimo: objetivo, critérios, decisões, restrições, etapa, IDs externos, SHAs, alterações existentes, checks, falhas abertas e próxima ação. Na retomada, verificar filesystem e fontes externas em vez de confiar no resumo [A1].

## 11. GitHub e autenticação

### Conexão visual

Botão **Conectar ao GitHub** abre autorização no navegador, com fluxo de dispositivo quando suportado e habilitado na aplicação OAuth cadastrada. Credenciais no Keychain do macOS, nunca no renderer, arquivos do projeto ou prompts. Escopos mínimos compatíveis com repos privados, PRs e Projects selecionados; mostrar permissões insuficientes e restrições organizacionais.

Botão **Conectar ao Codex** detecta instalação/autenticação suportada, mostra conta/estado quando expostos e orienta configuração. Não implementar formulário fictício de login.

### Efeitos e consistência

- Branch exclusiva `harnessy/<run-id>-<slug>`, baseada no SHA registrado.
- Núcleo publica estado de arquivos validado, sem confiar cegamente em metadados Git alterados pelo agente.
- PR contém tarefa, resumo, resultados de checks e identificação da execução.
- Marcadores/IDs de correlação e registro de intenção/resultado evitam duplicações após retry.
- Sem force push implícito ou merge automático.
- Revisão formal quando a identidade permite; mesma identidade do autor pode exigir comentário, identificado explicitamente.
- Inline somente em caminho/linha válidos; demais achados no corpo geral.
- Estados do Projects são mapeados por workspace, não nomes universais impostos ao board.
- Merge na UI exige ação humana, versão ainda validada, checks/regras GitHub e método permitido pelo repositório. Merge externo é reconciliado.

## 12. Ambiente de testes, CI e deploy

Assistente por workspace inspeciona o projeto e propõe imagem/runtime, instalação, serviços, checks, seed de dados, autenticação de teste, deploy e Playwright. Usuário confirma; estado de configuração distingue proposto, validado e falho.

**Padrão econômico:** testes locais primeiro; ambiente remoto compartilhado por workspace e deploy sob demanda. Uma reserva cobre deploy, confirmação de versão e sessão de validação. Renovação/expiração da reserva considera a execução real; lease expirado não autoriza sobrescrever um deploy ainda ativo sem reconciliação.

- Worktree não implica deploy.
- Hospedagem do projeto piloto ainda não escolhida; implementar um caminho real antes de generalizar provedores.
- Credenciais ficam no provedor/CI; Harnessy dispara workflow previamente autorizado com entradas restritas e referências imutáveis.
- Logs de CI são filtrados; não entregar dumps de ambiente aos agentes.
- Versão publicada deve ser verificável para todos os componentes relevantes; testar URL sem confirmar SHA não comprova a alteração.
- Tester usa conta/dados dedicados de teste, com credencial mediada e sem acesso aos segredos locais.
- Artefatos de navegador podem conter dados/autenticação: acesso restrito, filtragem e retenção configurável.
- Playwright combina testes determinísticos de regressão e exploração funcional orientada pelo agente; falha não deve ser “corrigida” apenas enfraquecendo o teste.
- Para o próprio Harnessy, testes de React no navegador não substituem validação de janela, IPC, autenticação, supervisor e lifecycle Tauri real.

## 13. Segurança: guardrails verificáveis

### Fronteiras

1. WebView: capacidades mínimas e IPC específico validado no Rust.
2. Núcleo confiável: credenciais, autorização e reconciliação de efeitos.
3. Executor do agente e descendentes: isolamento real de filesystem, rede, processos e ambiente.
4. CI/hospedagem: segredos de deploy, permissões e destinos próprios.

Worktree, `cwd`, instrução de prompt e capability Tauri não constituem, sozinhos, sandbox do shell [A4, O4, T1].

### Política obrigatória

- Sem acesso ao diretório pessoal, Keychain, configurações pessoais de nuvem/Git/SSH ou socket Docker.
- Workspace exportado por política de inclusão/exclusão; bloquear `.env` e variantes, chaves privadas, arquivos de autenticação e caminhos adicionais.
- `.env.example` somente com valores fictícios verificados. Considerar conteúdo e histórico Git, não apenas nomes.
- Resolver caminhos/symlinks e impedir escapes; Git interno de worktree não pode abrir acesso indireto ao clone/host com segredos.
- Shell e subprocessos obedecem às mesmas fronteiras que leitura/busca. Reduzir variáveis herdadas.
- Rede do executor limitada ao necessário; separar inferência autenticada do acesso de ferramentas. Não permitir exfiltração por acesso irrestrito como efeito de “autonomia”.
- Credenciais GitHub/deploy permanecem fora das sessões; ferramentas externas são capacidades específicas com validação de papel, estado, repo e destino.
- Conteúdo do repositório, páginas, MCP e stdout são dados não confiáveis e não ampliam política.
- Workflow versionado no repo não pode autorizar novos privilégios automaticamente; mudanças de permissões exigem confirmação na camada local confiável.
- Filtrar segredos conhecidos antes de logs/prompts/artefatos; não tratar redaction como barreira principal.
- Aprovações mostram ação, recursos e escopo; vinculadas à solicitação e invalidadas ao encerrar execução.
- Sem sandbox privilegiado ou montagem de socket Docker. Docker pode ser parte do runtime, mas a escolha final macOS precisa passar pelos testes de isolamento e autenticação Codex.

Falha de isolamento bloqueia execução; não oferecer fallback silencioso de acesso irrestrito. Testes devem cobrir acesso direto, shell, subprocessos, symlinks, histórico, ambiente, logs e tentativas de exfiltração.

## 14. Harness interno: contexto, eficiência e orçamento

O harness interno é um conjunto de políticas do núcleo, não um agente gerente executado em toda etapa.

- Contexto por tarefa/papel, recuperado just-in-time por referências [A2].
- Separar transcript de protocolo, estado operacional e resumo de retomada; não descartar itens opacos exigidos pelo runtime.
- Handoffs curtos com evidências consultáveis; compactação preserva decisões, restrições e falhas e é avaliada por testes de continuidade.
- Limitar saídas e manter artefatos completos sujeitos à retenção; evitar reenvio integral de logs.
- Checks determinísticos antes de nova análise de modelo.
- Reutilizar resultado apenas quando SHA, comando, configuração e ambiente relevante continuam válidos.
- Subagentes com objetivo limitado e justificativa operacional; contabilizar delegação, retries, resumos e avaliadores na tarefa principal.
- Limites de duração, turnos e consumo quando observáveis. Em Codex externo, não prometer controle do loop interno, cache ou teto de tokens que o protocolo não permita.
- Modelo por papel quando suportado; roteamento entre providers somente após adaptadores reais e evals comparativas.

### Contabilização

Separar **consumo observado**, **estimativa tarifária** e **cobrança reconciliada**. Ausência de informação significa desconhecido, não zero.

Registrar runtime, adaptador, modelo efetivo, modalidade de autenticação, IDs disponíveis, tokens por categoria, fonte do valor, latência e tarifa datada. Eventos cumulativos precisam de deltas reconciliados. Não contar cache ou reasoning duas vezes [O9].

Assinatura Codex não equivale a tarifação da API. Mostrar “equivalente estimado” somente se houver base válida, nunca como fatura. Encadear chamadas por ID não torna histórico gratuito em APIs [O7]. Cancelamento não desfaz consumo em voo.

Métrica principal: custo/consumo por tarefa validada, incluindo tentativas falhas. Acompanhar qualidade, intervenções, duração p50/p95, retrabalho e recuperação; otimização só é aceita sem regressão funcional relevante.

## 15. Configuração, SemVer e evolução

### Arquivos

```text
.harnessy/
  workspace.toml
  agents/
  workflows/
  checks/
```

Definições compartilháveis podem ser versionadas no repositório. Localmente ficam preferências pessoais, caminhos de clones, credenciais e decisões de confiança. Núcleo aplica precedência: política local confiável/limites globais → configuração aprovada do workspace → parâmetros permitidos da execução. Arquivo do repo nunca amplia autoridade local.

Configuração validada antes da execução, com schema, referências a segredos e detecção de incompatibilidade. Alteração durante execução vale para próximas execuções; a atual usa snapshot imutável. Configuração de workspace multi-repo tem origem canônica explícita, sem divergência silenciosa entre cópias.

### Versionamento

- Produto inicia em **0.1.0**, com fonte única para versão do núcleo, bundle e UI.
- PATCH: correção compatível; MINOR: funcionalidade compatível; MAJOR a partir de 1.0: mudança incompatível.
- Antes de 1.0, mudança incompatível exige ao menos MINOR e migração explícita.
- Pré-releases `0.2.0-alpha.1` / `0.2.0-beta.1`; releases com changelog, tag e artefatos identificáveis.
- Schema de banco/configuração, contratos, agentes e workflows possuem versões próprias e compatibilidade declarada.
- Cada execução registra versões/configurações efetivas; não reinterpretar histórico com instrução nova.
- Migrações transacionais quando possível, backup e política de recuperação; não abrir banco novo com binário incompatível silenciosamente.

### Após o build

Configurações, agentes, skills e workflows declarativos mudam sem recompilar, dentro das capacidades existentes. Mudança de Rust/React ou nova capacidade exige novo build/release.

Harnessy pode trabalhar no próprio repositório por tarefa → worktree → revisão → testes → PR → aprovação humana → release. Não reescrever o executável em execução. Separar dados e instalação estável/desenvolvimento para testes de autoaperfeiçoamento.

Distribuição futura requer assinatura/notarização aplicáveis e atualização com verificação de autenticidade e compatibilidade; nenhuma chave de provider compartilhada embutida no aplicativo.

## 16. Estado, persistência e observabilidade

SQLite mantém workspaces, vínculos, conversas, tarefas, runs, sessões, eventos ordenados, checks, artefatos, publicações, reservas, configuração/versionamento e pedidos de cancelamento.

Estados de execução: `queued`, `preparing`, `refining`, `implementing`, `checking`, `publishing`, `reviewing`, `waiting_environment`, `deploying`, `testing`, `awaiting_input`, `pausing`, `paused`, `ready_for_merge`; terminais `completed`, `no_changes`, `failed`, `cancelled`, `interrupted`.

Fluxos usam subconjunto de estados e transições validadas. Resultados de checks/review/teste são independentes do estado operacional. `ready_for_merge` exige gates aprovados; `completed` indica objetivo solicitado cumprido e não deve sugerir merge se ele não ocorreu.

Eventos incluem IDs, papel, timestamps, duração, versão e resultado; UI resume sem despejar prompts completos. Artefatos acessíveis apenas ao usuário local, com retenção e limpeza configuráveis. Não remover worktrees com alterações não preservadas silenciosamente.

Antes/depois de efeito externo: registrar intenção, correlacionar, confirmar e reconciliar. Não existe transação distribuída entre SQLite/GitHub/CI; estados parciais precisam ser representados.

## 17. Falhas e comportamento esperado

| Situação | Comportamento |
| --- | --- |
| Configuração, credencial ou runtime inválidos | Falhar antes de executar, com ação de resolução. |
| Rate limit/rede temporária | Backoff e retry limitados pelo prazo, reconciliando efeitos. |
| Preparação/baseline falhos | Preservar logs; não atribuir falha preexistente à mudança. |
| Check obrigatório falho | Bloquear validação de sucesso; PR draft quando publicado. |
| Review inconclusivo/reprovado | Sem deploy automático; pedir correção ou orientação. |
| Deploy falho/versão desconhecida | Não liberar tester como se a versão correta estivesse disponível. |
| Bug funcional | Criar/reconciliar card, handoff e ciclo de correção. |
| Três ciclos sem resolução | Pausar para decisão humana. |
| PR head mudou | Invalidar evidências afetadas e solicitar nova avaliação. |
| Cancelamento/crash | Preservar trabalho e publicações, tratar processos e reconciliar retomada. |
| Limite de contexto não observável | Handoff por evidências/sinais suportados, sem inventar percentual. |
| Scanner incompleto | Mostrar escopo/exclusões; não reportar auditoria integral. |

## 18. Avaliações e harness-score

### Avaliações comportamentais

Começar com fixture pequena de ponta a ponta e ampliar para conjunto representativo de tarefas reais (orientação Anthropic: 20–50 casos iniciais quando viável [A3]). Não bloquear a primeira prova técnica em volume artificial.

Trials com ambiente limpo e versões registradas. Graders determinísticos verificam estado final sempre que possível; julgamento por modelo exige calibração humana. Múltiplas tentativas medem variabilidade e consistência. Não exigir uma sequência exata de ferramentas quando soluções diferentes são válidas.

### Estratégia de testes por interface

Aplicar TDD em fatias verticais de comportamento crítico: um caso que falha pelo motivo esperado → implementação mínima → confirmação. Não escrever primeiro uma bateria inteira que espelhe uma arquitetura imaginada. Acordar os seams da fatia antes de implementar; a tabela abaixo é a proposta a confirmar nesse momento, não aprovação implícita para toda cobertura possível.

| Seam proposto | O que comprova | O que não comprova sozinho |
| --- | --- | --- |
| Interface de execuções | Gates, cancelamento, limites e retomada observáveis | Protocolo e sandbox reais do Codex. |
| Adaptador Codex com fixtures do protocolo fixado | Correlação, eventos, erros e compatibilidade dos contratos | Autenticação/execução real do runtime. |
| Publicações contra servidor de teste e piloto GitHub | Reconciliação e ausência de duplicação em falhas | Todas as políticas de uma organização GitHub. |
| Executor com arquivos-canário fictícios | Negação de caminhos, rede e processos proibidos | Ausência de vulnerabilidades desconhecidas. |
| Fluxo desktop real | UI, IPC, lifecycle e entrega funcional integrada | Todas as combinações de SO e carga. |

Testes cruzam as mesmas interfaces dos chamadores, sem asserts em métodos privados, contagem de chamadas internas ou estado duplicado. Usar valores esperados derivados dos requisitos, não recomputados pelo algoritmo testado. Doubles substituem efeitos externos nas suítes rápidas; testes de contrato e piloto validam as integrações reais. Dados-canário não usam credenciais verdadeiras.

Para correção de bug, primeiro obter reprodução que detecte o sintoma original; preservar como regressão no seam adequado quando viável. Teste verde por exclusão de cenário, relaxamento sem justificativa ou referência trocada não comprova correção. Separar falha funcional, teste flaky e infraestrutura.

Cobertura mínima: tarefa simples; multi-repo; ambiguidade/refinamento; bug/reteste; crash/handoff; cancelamento/negação; efeitos duplicados; head alterado; saída/contexto grandes; concorrência; traversal/symlinks; prompt injection; acesso a credenciais; rate limit; ambiente remoto ocupado.

### Prontidão estática

Há dois OSS identificados como `harness-score`, sem confirmação de qual o usuário pretendia:

- **paladini/harness-score:** scanner estático CLI de maturidade do repositório; referência candidata por trabalhar sem LLM durante o scan segundo sua documentação [H3].
- **seanrobertwright/harness-score:** workflow Archon com detector e relatório por Claude; distinto e não adequado como dependência inicial somente-Codex [H5].

Adotar como boas práticas: instruções navegáveis, skills focadas, hooks/checks mecânicos, sensores/testes, CI e higiene de segredos. Não criar arquivos vazios apenas para aumentar pontuação.

Integração do scanner é opcional e posterior: fixar versão/modelo de scoring, verificar cobertura Cargo/Tauri, executar sobre cópia filtrada e registrar exclusões. Não dar acesso a arquivos privados para obter nota maior. Nota alta não comprova correção, segurança ou qualidade de testes. Sem instalação/execução do scanner nesta pesquisa.

## 19. Etapas de entrega

1. **Consolidação e migração:** SPEC única; substituir protótipo Electron por Tauri/Rust/React; versão 0.1.0 centralizada e comandos reproduzíveis.
2. **Fatia de UX/performance:** navegação, board/chat/contexto, estados reais, teclado e fixtures de carga; validar build desktop.
3. **Núcleo:** SQLite, configuração versionada, contratos, fila, supervisor, eventos e recuperação.
4. **Prova Codex/segurança:** app-server, schemas, autenticação, eventos, isolamento, aprovações, cancelamento e retomada. Gate antes de agentes em projetos reais.
5. **GitHub:** OAuth/Keychain, Projects/cards, PR/review/merge humano, reconciliação e idempotência.
6. **Agentes e trabalho:** refinador/implementador/revisor/tester, worktrees, subagentes, handoffs, limites e instruções em andamento.
7. **Validação funcional:** setup assistido, dados/login de teste, CI/provedor piloto, reserva/deploy/Playwright e ciclo de bug.
8. **Release:** suite de evals, regressões de desempenho, migrações, changelog e preparação para distribuição.

Estado atual do repositório: fundação **Tauri 2/Rust/React/TypeScript**, com navegação, tokens, rascunho local e IPC restrito a metadados. Build macOS gerado; sem integrações GitHub/Codex, SQLite ou sandbox de agentes. Metas de desempenho e fixtures de carga ainda precisam ser medidas. Esta especificação distingue requisitos futuros de recursos implementados.

## 20. Critérios de aceite

O MVP funcional está pronto quando demonstra em projeto piloto:

1. Abrir aplicativo macOS com UX/performance medidas nas metas acordadas.
2. Conectar GitHub/Codex pela interface, sem gravar credenciais no repo/renderer/conversa.
3. Consultar/editar board real e reconciliar alterações externas.
4. Refinar tarefa ambígua, confirmar critérios e iniciar execução explicitamente.
5. Executar Codex real em isolamento comprovado, com papéis independentes e contexto adequado.
6. Produzir alterações isoladas e checks sobre os SHAs publicados; suportar worktrees e delegação dentro dos limites.
7. Criar PR/review ou comentário sem duplicação, reportando falhas e resultados com precisão.
8. Publicar versão revisada no ambiente piloto, confirmar SHA e testar com Playwright.
9. Bug gera card/handoff; correção passa por revisão/deploy e reteste independente.
10. Marcar pronto para merge só com gates válidos; merge humano na UI ou externo reconciliado.
11. Pausar/parar/retomar e fechar janela sem perder estado nem alegar rollback de efeitos.
12. Recuperar de crash sem repetir publicação; sucessor continua com restrições e evidências preservadas.
13. Bloquear acesso a `.env`, credenciais locais e recursos proibidos também via shell/subprocessos/rede nos testes definidos.
14. Registrar versões, consumo disponível/desconhecido, limites e custos sem dupla contagem ou equivalência falsa com assinatura.
15. Atualizar configuração declarativa sem build e aplicativo por release SemVer, com compatibilidade/migração verificadas.
16. Passar checks Rust/TypeScript, testes de contratos/estados/segurança/idempotência e validação do aplicativo Tauri real; testes web não bastam para integração desktop.

## 21. Referências principais

- [A1] Anthropic: [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents).
- [A2] Anthropic: [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents).
- [A3] Anthropic: [Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents).
- [A4] Anthropic: [Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing).
- [O1] OpenAI: [Harness engineering](https://openai.com/index/harness-engineering/).
- [O2] OpenAI: [Codex app-server](https://developers.openai.com/codex/app-server).
- [O4] OpenAI: [Codex sandboxing](https://developers.openai.com/codex/sandboxing.md).
- [O7] OpenAI: [Conversation state](https://developers.openai.com/api/docs/guides/conversation-state.md).
- [O9] OpenAI: [Pricing](https://developers.openai.com/api/docs/pricing.md).
- [H3] [paladini/harness-score](https://github.com/paladini/harness-score).
- [H5] [seanrobertwright/harness-score](https://github.com/seanrobertwright/harness-score).
- [T1] Tauri: [Capabilities](https://v2.tauri.app/security/capabilities/).

Fontes mutáveis consultadas em 2026-10-29; revalidar schemas, versões, permissões e preços na implementação. Pesquisa não equivale a auditoria ou benchmark executado.

## 22. Skills aplicadas e rastreabilidade

| Skill consultada | Aplicação direta nesta especificação |
| --- | --- |
| `research` | Fontes primárias, relatório de evidências e limites das alegações (§2, §18, §21). |
| `find-skills` | Descoberta e verificação da skill externa React, sem instalação automática. |
| `codebase-design` | Módulos profundos, interfaces pequenas, invariantes e seams (§6). |
| `design-system` | Tokens em três camadas e contratos/estados dos componentes (§5). |
| `ui-ux-pro-max` | Minimalismo funcional, teclado, foco não obscurecido e acessibilidade (§5). |
| `tdd` | Fatias verticais, seams a confirmar e testes de comportamento (§18). |
| `implement`, `code-review`, `diagnosing-bugs` | Inspiração já examinada para contratos de implementação, revisão e reprodução (§7–8). |
| Vercel `vercel-react-best-practices` | Regras selecionadas para carregamento, listeners e prioridade de atualização (§5). |

A busca `npx skills find react performance` retornou a skill oficial Vercel com aproximadamente 851,8 mil instalações; repositório verificado com aproximadamente 33,2 mil estrelas em 2026-10-29. Popularidade é sinal de descoberta, não comprovação técnica. Foram lidos o [SKILL.md](https://github.com/vercel-labs/agent-skills/blob/main/skills/react-best-practices/SKILL.md) e as regras `rerender-transitions`, `bundle-conditional` e `client-event-listeners`. Aplicaram-se os princípios pertinentes, sem exigir SWR, Next.js ou copiar alegações de SSR para Tauri.

As skills orientam decisões e verificações; não substituem evidência de execução nem autorizam mudanças de escopo. Não houve instalação de skills externas nesta revisão.
