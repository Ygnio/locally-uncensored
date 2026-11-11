# Avaliadores da memória 2.2F

Os executores da Fase 2.2F usam raízes marcadas, o bridge dedicado e um manifesto que sela código, corpus e modelos. Cada execução recebe um rótulo exclusivo. Os artefatos completos ficam na pasta privada `reports` do QA. O progresso registra casos em JSON por linha e o relatório final preserva os casos previstos, inclusive os bloqueados por pré-condições.

## Recuperação C1 e C2

C1 restaura relações revisadas sintéticas pela API existente `Store::restore_isolated`, valida o estado canônico e usa as APIs de exclusão, representação, vetores e recuperação do produto. Os registros da fixture têm fonte `reviewed_import`. O executor não chama o pipeline de extração em C1. Um cenário contém um banco e seus vínculos de escopo e projeto.

C2 usa uma resposta de extração do Gemma para cada fonte USER. O bridge entrega a fonte ao pipeline nativo híbrido, verifica o job e a prova de origem, aplica revisão quando indicada e indexa registros ativos com Nomic. Para cada consulta híbrida com pré-condições válidas, o processo Node usa o adaptador de contexto e o verificador da aplicação com Gemma real. Esse caminho mede a integração de memória e resposta pelo bridge. A execução real do composer, do DOM e do transcript pertence à avaliação GUI. O relatório guarda o texto entregue, as gerações brutas, os usos e a latência para revisão literal independente.

Cada consulta declara as sementes e operações necessárias em `requires`. O conjunto inclui os registros esperados, os proibidos e a operação de exclusão quando a consulta depende de um tombstone. Uma falha de preparação mantém seus dependentes como `not_executed` e permite os demais. Precisão e recall consideram apenas consultas medidas, com cobertura e recall no plano integral informados separadamente. As metas automáticas exigem cobertura completa e o C2 exige também revisão literal das respostas.

Os modos registram a seleção nativa com peso lexical 1, 0 e 0,5, respectivamente FTS5, Nomic e híbrido. A trilha expõe BM25, cosseno e referências efetivamente injetadas. O fallback usa a política híbrida e a ausência de vetor de consulta. A variante `--offline` verifica a porta física do Nomic fechada e executa somente o fallback C1, em outro banco.

Um plano inédito declara `scenarios`, com `id`, `events` e `queries` por cenário.

- Uma semente C1 contém `action: "seed"`, `label`, `scope`, `project` e `fact`. O fato tem `kind`, `subject`, `property`, `value`, `temporal` e `content`. `facets` e `fact_key` são opcionais. Uma versão histórica usa `state: "obsolete"` e carrega a revisão ativa imediatamente anterior, conforme a exigência da recuperação nativa. A versão atual pode apontar para uma semente anterior em `replaces`.
- Uma fonte C2 contém `action: "learn"`, `label`, `scope`, `project`, `text` e os mesmos campos tipados em `fact`, exceto `content`. A revisão explícita ocorre se necessária e `approve` permite registrá-la no plano.
- Uma exclusão ou obsolescência contém `action: "delete"` ou `"obsolete"`, `label` e `target` referente a uma semente anterior.
- Uma consulta contém `id`, `query`, `scope`, `project`, `requires`, `expected` e opcionalmente `forbidden`, `mode` e `lexical_mandatory`. `expected` e `forbidden` referenciam rótulos de sementes.

As anotações de desenvolvimento leem os 17 eventos e as 29 consultas originais da 2.2E, com 20 obrigatórias para fallback. O manifesto de desenvolvimento deve incluir as anotações e o corpus original. [Anotações de recuperação](../crates/lucille-memory/tests/fixtures/phase22f/development/known-retrieval-annotations.json).

## Falhas e transporte

O plano contém uma lista `cases` para falhas e uma lista `transport` com um caso por modo. Cada caso de transporte fornece `mode`, `project`, `text` e `expected`, incluindo `kind`, `subject`, `property`, `value` e opcionalmente `state`. A avaliação confere o erro real exibido, a mensagem USER única, a identidade, o recibo posterior, a versão canônica e a proveniência da mesma fonte. A expectativa do Agent para tema escuro é validada como tema escuro.

Os casos de falha suportam `prepared_cancel`, `ordered_sources`, `leased_cancel`, `restart` e `sqlite_busy`. A ordenação requer duas fontes e o `expected` mais novo. O reinício requer a lista `expected` de fatos. O teste SQLite adquire e desfaz um lock de escrita no banco sintético sem modificar dados e verifica a rejeição pela API nativa. Os cenários de lease e transporte executam no host isolado, com conclusão tardia baseada na resposta real do Gemma. O teste de ordem registra o estado antes e depois da fonte antiga e aceita histórico distinto quando o canônico atual se preserva.

As anotações para desenvolvimento conservam as fontes originais da 2.2E e vinculam cada uma ao fato esperado. [Anotações de protocolo](../crates/lucille-memory/tests/fixtures/phase22f/development/known-protocol-annotations.json).

Os executores aceitam `--root`, `--freeze`, `--dataset` e `--label`. Recuperação também recebe `--track c1` ou `--track c2`. Os controladores de GUI compartilham o lock do host e devem rodar em sequência. [Recuperação](../scripts/test-lucille-phase22f-retrieval.py), [falhas](../scripts/test-lucille-phase22f-faults.py), [transporte](../scripts/test-lucille-phase22f-transport.py).
