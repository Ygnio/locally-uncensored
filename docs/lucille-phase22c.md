# Lucille — Fase 2.2C

**Resultado parcial: fase reprovada pelos critérios congelados.** Implementação, regressões, avaliação real e evidências foram concluídas em QA isolado. Sem merge, release ou instalação operacional. A autonomia operacional permanece desligada.

## Versões e protocolo

- Base local/remota conferida antes das alterações: `97d610e7b508dbf5de9d964be56c41254f739ec7` (PR #10).
- Branch: `feature/lucille-phase22c-evidence-authority`; [PR #11 em rascunho](https://github.com/Ygnio/locally-uncensored/pull/11), empilhado sobre #10.
- Código encerrado: `0e9106f0c2bf061c66fb32cbfb8fce420c4bc67b`, antes da criação dos dados finais.
- Dados inéditos commitados em `dcbc3b98`; congelamento publicado em `847b9dae1d3a7e3b192b896e775e8848cbc2e7a5`, antes de qualquer execução final.
- [Manifesto congelado](../crates/lucille-memory/tests/fixtures/phase22c/final-v1/freeze.json): fontes, prompts, política, modelos/runtime, schema/representação/índice 5, protocolo 3, executáveis, frontend, corpora, rubrica e limites.
- O HEAD final de publicação e sua auditoria de CI são registrados no relatório local e no corpo do PR depois do último commit, evitando alterar o próprio SHA auditado.

Nenhuma alteração de código/corpus/limiar/prompt foi feita após observar a avaliação inédita. As falhas permaneceram preservadas. A matriz anterior às implementações e as três rodadas de desenvolvimento estão em [design e diagnóstico](lucille-phase22c-design.md) e [evidências de desenvolvimento](evidence/phase22c/development).

## Implementação

A verificação separa entidade, propriedade, valor e modalidade de cada afirmação de maior risco. Decisões, histórico, operações de memória e observações de ferramentas não concedem a mesma autoridade. Resultado de ferramenta inclui operação, recurso, resultado, momento e cobertura limitada; o texto bruto da ferramenta não é promovido ao envelope SYSTEM de ambiente verificado.

As coordenações com novos predicados preservam modalidades independentes. A recuperação histórica composta foi corrigida sem reduzir o limiar 0,6. O parser reconhece a descrição “continua sendo”; fonte V3 deterministicamente inelegível termina sem repetir três vezes, enquanto falhas transitórias mantêm o limite anterior. Não houve gravação forçada nem aumento indiscriminado de retry. A persistência da fonte é descrita por estados reais; não se presume descarte de texto apenas por revogar a lease.

O reparo localizado preserva frases independentes literalmente, com original, remoções e nova verificação, quando todos os elementos pedidos permanecem. Caso contrário há no máximo uma regeneração real. Uma resposta retida continua sendo perda de disponibilidade. A projeção está coberta por testes; **nenhuma ocorreu na rodada inédita**, portanto não se apresenta como sucesso observado de edição real nessa rodada.

Chat/Agent/Code usam a identidade compartilhada e a pergunta original para resolver o idioma antes da concatenação com memória portuguesa. Perguntas estritamente de memória recebem tratamento sem inspeção externa; ações explícitas mantêm ferramentas. Narração do Agent, texto exibido e cópia posterior preservam ID e corpo verificado. Os reconhecedores são finitos: esta implementação não é um oráculo universal de linguagem natural.

## Avaliação inédita

40 casos de extração (24 fatos positivos e 16 entradas negativas), 28 consultas de recuperação e três conversas novas com 43 turnos, 45 relatórios únicos e 50 gerações de resposta reais. Todos os dados são sintéticos. As sondas criam sessões novas sem histórico anterior.

| Critério | Resultado | Conclusão |
|---|---:|---|
| Extração — precisão | 16 TP / 17 propostas aceitas = **94,12%** | Abaixo de 95% |
| Extração — recall | 16 / 24 = **66,67%** | Abaixo de 80% |
| Gravação crítica indevida | 1 caso negado virou candidata em cada modo | Reprovado; não houve promoção automática dessa candidata |
| Promoção automática | 11 corretas, 0 incorretas por modo | Isoladamente passou |
| Híbrido — precisão / recall | 21 TP, 2 FP, 3 perdas = **91,30% / 87,50%** | Mínimos numéricos atingidos, mas critérios negativos falharam |
| Relevância e ator | 2 injeções irrelevantes; 1 envolve pergunta sobre irmão recebendo fato do projeto do usuário | Reprovado por vínculo semântico de ator |
| Namespace/projeto ou excluída como atual | 0 nos contadores nativos | Não equivale a ausência de erro semântico de ator |
| Fidelidade entregue | **44/44 afirmações sustentadas**, 0 críticas | Passou somente neste conjunto inédito |
| Disponibilidade útil | **38/43 = 88,37%** | Abaixo de 95% |
| Completude | **42/48 elementos = 87,50%** | Abaixo de 95% |
| Completude apenas entre entregues | 93,33% | Medida suplementar; não oculta retenções |
| Notas do Agent / cópia | 1 nota observada no DOM e 1 cópia com metadados íntegros | 0 notas exigidas sem verificação; 0 perdas no fluxo observado |
| Idioma | 0 desvios entre respostas entregues | Passou no conjunto |

Os três modos de extração repetem o processamento nativo das **mesmas 40 respostas reais** do Gemma; não são 120 inferências independentes. Nos oito fatos perdidos, o Gemma indicou a evidência correta, mas a normalização nativa não a transformou em registro. No contraexemplo negado, ambos indicaram/aceitaram evidência que não sustentava a quantidade de RAM; a revisão impediu ativação, mas a escrita como candidata já viola o critério.

As perdas úteis incluem uma preferência cuja nova redação não reativou o tombstone, um assunto de catálogo não recuperado, uma ausência correta de GPU falsamente bloqueada e um plano MySQL cujo turno foi retido por falta de representação da fonte atual. O cancelamento desse último turno prejudicou consultas posteriores. Ausência de contexto não comprova que o usuário nunca informou ou decidiu.

A revisão independente lê cada relação e fonte original, sem usar o veredicto da aplicação como verdade. Perguntas, conselho técnico e conhecimento geral são separados de afirmações de memória. Uma ausência delimitada de registro pode estar literalmente correta e ainda ser uma não resposta inútil. [Revisão literal](evidence/phase22c/final-v1/claim-ledger.json), [utilidade](evidence/phase22c/final-v1/utility-ledger.json), [recálculo público](evidence/phase22c/final-v1/public-rescore.json) e [atribuição de falhas](evidence/phase22c/final-v1/failure-attribution.json).

## Segurança, custo e limites da resposta

Nas 50 gerações, a revisão encontrou 54 afirmações: 49 sustentadas, 3 extrapolações críticas brutas e 2 não críticas brutas. Nenhuma dessas cinco indevidas foi entregue na rodada inédita. Houve três respostas bloqueadas e duas regenerações aceitas. Os bloqueios também incluíram conteúdo correto: prevenir erro não compensa reter utilidade.

Foram cinco gerações extras reais: 12,563 s, 7.125 tokens de entrada e 112 de saída adicionais; refresh 564,2 ms e verificação 51,5 ms somados sobre 45 relatórios únicos. Cópias ocultas não duplicam custos. [Custos](evidence/phase22c/final-v1/intervention-cost-final.json).

Payloads encaminhados e respostas reais estão preservados, com hashes das partes privadas de SYSTEM. O broker adaptou a conclusão real não streaming para SSE. **Streaming nativo do modelo não foi validado.** [Payloads](evidence/phase22c/final-v1/actual-forwarded-payloads.json).

## Regressões longitudinais

Executadas uma vez no escopo compartilhado `phase22c-final-historical-series`, na ordem original 20 → 50 → 100, projetos `candeia` → `candeia2` → `candeia`. Sem o erro anterior de criar um escopo vazio diferente para 100. A série contém 170 turnos de conversa e 28 sondas adicionais.

| Série | Recuperação estrita P/R | Sondas entregues | Expressão antiga | Fidelidade literal de memória, diálogo + sondas |
|---|---:|---:|---:|---:|
| 20 | 100% / 100% | 10/10 | 10/10 | 14/16; duas extrapolações não críticas de desempenho por RAM |
| 50 | 100% / 100% | 8/8 | 7/8 | 14/14; ausência correta em escopo de outro projeto não reconhecida pela expressão antiga |
| 100 | 90,91% / 100% | 10/10 | 10/10 | 20/21; uma inferência crítica de desenvolvimento em curso |

Os três diálogos tiveram um turno inicial retido cada, também preservado. A série 100 retornou um registro real adicional de estratégia por turnos para recomendação de RPG; o gold antigo só pede tática. O FP estrito permanece, sem alterar o gold. A resposta final de projeto em 100 diz “você está desenvolvendo”; a descrição original apenas informa começo de projeto, e linguagem/banco são escolha/plano. A política estrita não autoriza elevar isso a execução atual. A fonte original foi preservada para interpretação pelo revisor. Isso **reprova o requisito de zero extrapolação crítica nas regressões**, mesmo com todos os valores esperados presentes.

[Revisão histórica completa](evidence/phase22c/final-v1/historical-literal-review.json). Os testes de 200 turnos e de 62.545 tokens não foram repetidos; o primeiro era opcional, o segundo foi explicitamente dispensado sem nova justificativa.

## Falhas, persistência e recursos

Passaram: cancelamento antes de publicar; cancelamento de lease em execução; rejeição de resposta real tardia do Gemma; recuperação automática após reinício em extração, na segunda tentativa e sem duplicação; ferramenta em pasta vazia; falha real de ferramenta sem concluir ausência de arquivos do projeto; falha do transporte de geração com cancelamento da fonte e zero gravação; persistência em novas sessões. Uma tentativa de controle de QA enviou campos extras à API estrita, e outra omitiu confirmação; foram preservadas e corrigidas no controle antes de encerrar desenvolvimento. Não foram contadas como falhas do produto ou cancelamentos bem-sucedidos.

**Nomic offline: resultado parcial.** Com o servidor próprio desligado, o fallback lexical não recuperou os 48 GB conhecidos. Restaurado o mesmo Nomic e repetida a pergunta idêntica, recuperou 48 GB sem alterar a memória canônica. O mecanismo de fallback existe, mas sua utilidade não passou neste caso. A falha do transporte do modelo não é apresentada como um crash real do RPC nativo Tauri; esse limite permanece explícito.

122 bases isoladas (perfil GUI e replays finais) passaram em `integrity_check`, chaves estrangeiras e fila/leases. No perfil completo: 0 vetores atuais obsoletos, 0 vetores órfãos, 0 fingerprints ativos duplicados. Nenhum job pendente/running/retry ou lease ativa no conjunto auditado. Tombstones, replay/deduplicação, correções, conflitos, migrações, concorrência, backup/restore, reindexação e erros de bridge permanecem cobertos pela suíte técnica; não se apresenta cada teste unitário como um novo ensaio real independente de falha.

| Recursos na avaliação final | A: medido | B: projeção somente |
|---|---:|---:|
| Menor RAM física disponível | 1.389 MiB | 3.437 MiB com +2.048 MiB hipotéticos |
| Maior commit do sistema | 21.920,9 MiB | Sem alteração assumida |
| Menor folga de commit | 10.802,5 MiB | Sem alteração assumida |
| Uso atual de pagefile | 1.009–1.042 MiB | Não alterado |
| VRAM máxima do dispositivo | 9.500 MiB | Sem ganho medido inferido |

O pico histórico de pagefile de 8.007 MiB é anterior à janela e não foi atribuído a este ensaio. CPU e GPU globais incluem outros aplicativos. Gemma atingiu 7.548,9 MiB de working set e 9.339,7 MiB de commit privado; Nomic 790,1/932,5 MiB. O sampler de GUI não inclui toda a árvore WebView. Foram 343 chamadas Gemma e 344 Nomic na janela, 718.572 tokens de prompt e 15.046 de conclusão; maior prompt observado 6.716 tokens, contexto **configurado preservado em 65.536**. Latência Gemma P50 3,72 s/P95 11,60 s, incluindo respostas e extração. [Recursos e limitações](evidence/phase22c/final-v1/resources-final.json).

## Verificações técnicas e encerramento

- 126 testes Rust, incluindo os 122 anteriores; 0 falhas/ignorados.
- 687 TypeScript da suíte de memória e regressões, incluindo os 666 anteriores; 0 falhas/pulados.
- 3 testes de esquema atômico; typecheck; ESLint dos arquivos alterados; builds do frontend QA, host nativo dedicado e bridges.
- Hashes dos executáveis e frontend no congelamento; evidências sintéticas com manifesto SHA256 próprio.
- Processos próprios Gemma/Nomic/broker/GUI/Vite/sampler encerrados, portas 15283/15284/15287/15288/15289 fechadas. Encerramento por identidade verificada; código 1 dos supervisores significa parada de teste solicitada, não falha por pressão.
- Sem instalação operacional, perfis/conversas/bancos reais, modelos originais, launchers, drivers, pagefile ou portas operacionais alterados. Nenhum aplicativo existente encerrado. Sem merge/release.

O CI remoto tem auditoria limitada a uma tentativa no HEAD final publicado; não há declaração de verde. O bloqueio HTTP 422 anterior e a ausência de runs não são equivalentes a testes aprovados. A auditoria final exata fica fora do Git e no PR para não criar um ciclo de autoalteração de SHA. Nenhuma configuração administrativa foi modificada.

A próxima correção deve continuar na memória: negação e paráfrases na normalização, relevância por ator/propriedade, reativação de tombstones, verificação da fonte atual e modalidades de execução em formas linguísticas adicionais. Será necessário outro conjunto independente após esse desenvolvimento; este conjunto final permanece reprovado e imutável.
