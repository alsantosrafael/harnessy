# Medição — harness econômico

Mede o ganho do harness para **cada** modelo ([spec §8](../../docs/specs/harness-economico.md)): o mesmo modelo cru contra ele mesmo com harness, aprovado por **testes ocultos** que o agente não vê.

## Tarefas

Cada pasta em `tasks/` tem:

| Arquivo | Papel |
| --- | --- |
| `task.md` | Título (`# …`), descrição e `## Critérios de aceitação`, como uma Tarefa do app |
| `repo/` | Repositório inicial (vira um git novo a cada run) |
| `hidden/` | Testes ocultos, sobrepostos ao resultado só na hora de aprovar |
| `solution/` | Solução de referência; o teste `tasks_with_hidden_tests_load_and_their_reference_solutions_pass` confere que ela passa nos ocultos e que o repositório sem mudança falha |

Hoje são 4 tarefas reais com alteração em mais de um arquivo (feature, bug, validação, formato). A spec pede pelo menos 10: faltam refactor, critério ambíguo, teste quebrado no baseline e tentação de enfraquecer teste. As 8 tarefas de `experiments/loop/tasks` entram como sanidade com `HARNESSY_ECON_SANITY=1` (sem ocultos: aprovada = Pronto).

## Rodar

Chama o `claude` real e consome a sua conta. Com 4 tarefas, 3 modelos, 3 condições e 3 tentativas são 108 runs.

```sh
cd src-tauri
cargo test economico -- --ignored --nocapture
# recortes
HARNESSY_ECON_MODELS=haiku HARNESSY_ECON_TRIALS=1 cargo test economico -- --ignored --nocapture
HARNESSY_ECON_CONDITIONS=cru,harness HARNESSY_ECON_ONLY=cupom cargo test economico -- --ignored --nocapture
```

Condições: `cru` (uma chamada, sem loop, sem contraprova, sem arquivos prováveis), `harness` (loop, falha estruturada, contraprova, arquivos prováveis) e `revisor` (harness + revisor um degrau acima, só com sinal de risco).

Cada execução acrescenta uma seção a `RESULTADOS.md` com data, versão do back e da CLI, a tabela por célula, o ganho do harness por modelo e o detalhe por tarefa.
