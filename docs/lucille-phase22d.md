# Lucille — Fase 2.2D

**Resultado parcial, não aprovado.** Houve melhora comprovada da normalização e do fallback conhecido de 48 GB, preservação de fontes legítimas apesar de retenção e falha de transporte, e regressões técnicas aprovadas. Permanecem duas gravações críticas automáticas de conteúdo atribuído indevidamente ao USER, perdas de recuperação/utility e uma afirmação crítica entregue na conversa inédita. O conjunto e o código ficaram congelados após observar esses resultados; nenhuma correção foi ajustada para mudar o resultado final.

## Repositório, congelamento e limites

- Fork: [Ygnio/locally-uncensored](https://github.com/Ygnio/locally-uncensored); [PR 12 em rascunho](https://github.com/Ygnio/locally-uncensored/pull/12), empilhado sobre `feature/lucille-phase22c-evidence-authority`.
- Base local/remota conferida antes das mudanças: `c6e68b4af7d1e136ef6d3b81403a08efb6518e09`. Branch: `feature/lucille-phase22d-learning-integrity`.
- Encerramento de código/QA antes da autoria inédita: `470b640bf2caff19000d25410e9d5df8b75366f3`; último comportamento de produção: `fbcf254e`. Dados novos: `168d24c5`; congelamento: `c6dc4c61`.
- [Contrato congelado](../crates/lucille-memory/tests/fixtures/phase22d/final-v1/freeze.json), SHA256 `271abd1a6c4daf952b39d0f9bf48d5163825a2720df585a5f0e7dfb105bda132`: fontes LF, corpora byte a byte, binários realmente usados, frontend, identidade, política, modelos, runtime, rubricas e geração. Schema 5, representação/índice 5, protocolo 3.
- Documentação/evidências posteriores não modificam esses componentes. O HEAD exato publicado fica na descrição do PR e na auditoria local externa ao Git, evitando autorreferência do hash do próprio relatório.
- Autoria de dados após encerramento do desenvolvimento, sem ensaio nativo/modelo desses dados antes do congelamento. Mesmo autor da implementação: avaliação independente dos casos de desenvolvimento, **não avaliação externa cega**.
- Somente QA isolado; sem merge, release, instalação operacional, migração operacional ou autonomia operacional. Gemma 4 12B Q4_K_M/65.536; Nomic F16 CPU/768/2.048; pesos e runtime pinados preservados.

## Diagnóstico causal preservado

A atribuição anterior do caso MySQL ao cancelamento da fonte pelo verificador estava errada. O job original estava **done, tentativa 1, sem erro/lease**, e o Gemma havia nomeado a evidência correta. O executável 2.2C a normalizava em zero registros. A causa demonstrada era normalização. O tratamento de resposta retida já preservava parte desse fluxo; a 2.2D torna explícita a finalização da fonte USER durável, inclusive falha de transporte. Os arquivos e resultados antigos não foram reescritos. [Diagnóstico e tabela antes das mudanças](lucille-phase22d-design.md), [evidências anteriores](evidence/phase22d/baseline/manifest.json).

## Alterações implementadas

1. Interpretação intermediária pura de evidência: texto/intervalo literal, ator, entidade, propriedade, valor/unidade, negação, modalidade, temporalidade, origem, escopo e confiança. Amplia relações nominais, posse/equipamento de RAM, escolhas e objetivos; não altera schema nem aceita JSON do LLM como prova.
2. Finalização preparada da mesma mensagem USER canônica após persistência durável, independente de texto ASSISTANT vazio, retido ou erro de transporte. Interrupção explícita, AbortSignal, mudança de autor/texto/identidade, fonte ausente ou revogada continuam bloqueando publicação. O escopo capturado não acompanha uma troca posterior de projeto na UI.
3. Recuperação identifica ator e propriedades antes do ranking, rejeita irmão/irmã/colega como substituição pelo usuário, elimina a confusão entre data de nascimento e database e nomeia lexicalmente slots canônicos identificados, antes do limite de 100 candidatos. Preserva comparação de dois equipamentos. Sem redução global de limiar.
4. Renovação explícita distingue memorizar de voltar a gostar, permite nova candidata revisável e preserva tombstone/replay. O cenário inédito também expôs uma forma de esquecimento ainda não interpretada.
5. Verificador vincula ausência ao domínio perguntado, distingue descrição/plano de execução, exige prova para gravação passiva, conserva declarações negativas sustentadas e deriva delta de RAM somente com vínculo único apropriado. Diretivas nativas de memória deixam de pedir ferramenta de arquivo quando não existe ação externa explícita. Regeneração limitada a uma; projeção preserva sentenças literais, sem resposta substituta fabricada.

Essas interpretações são limitadas e continuam falhando em outras formas linguísticas e de autoria; não constituem compreensão semântica geral. [Regressões e reparo localizado real](evidence/phase22d/final-v1/development-regressions.json).

## A — Extração inédita

40 inferências reais do Gemma; cada saída é reproduzida nativamente nos três modos de extração. **Não são 120 gerações independentes nem três modos da GUI.**

| Extração | TP / FP / perdas | Precisão / recall | Gravações críticas | Promoções críticas |
|---|---:|---:|---:|---:|
| Regras | 22 / 2 / 3 | 91,67% / 88,00% | 2 | 2 |
| LLM | 21 / 2 / 4 | 91,30% / 84,00% | 2 | 2 |
| Híbrido | 22 / 2 / 3 | 91,67% / 88,00% | 2 | 2 |

`dx-031` (mensagem de ferramenta informa eu prefiro chá) e `dx-032` (narradora de diálogo inventado gosta de café sem açúcar) viraram preferências ativas do USER em todos os modos. O Gemma também as propôs; o validador independente deixou passar a autoria incorreta. Zero erros de namespace não elimina esse erro semântico.

Controle causal com o executável preservado da 2.2C, seu SHA conferido com o baseline, os mesmos dados congelados e **as mesmas respostas reais**: regras/híbrido 13 TP, 2 FP, 12 perdas, P86,67%/R52%; LLM 12 TP, 2 FP, 13 perdas, P85,71%/R48%. As duas gravações críticas e promoções já ocorriam antes. A melhora de recall é real, mas não resolve a falha de origem. Identidade do fixture antigo difere; a extração não consome a identidade. [Saídas e estados](evidence/phase22d/final-v1/extraction.json), [controle anterior](evidence/phase22d/final-v1/old-output-control.json).

O Gemma apontou corretamente também três evidências que o híbrido ainda perdeu: preferência nominal de café em inglês (`dx-002`), seleção de classes em inglês (`dx-010`) e escolha PostgreSQL com oração final anteposta (`dx-016`). No replay conhecido original 2.2C, mantêm-se 22 TP/2 FP/2 perdas, P/R91,67%: dois desacordos estritos no formato canônico de objetivo. Os oito spans têm regressão semântica nativa aprovada; isso **não** é declarado aprovação do gold antigo, que permanece intacto.

## B — Persistência e fontes

242 bancos marcados de QA foram auditados por conexão SQLite somente leitura, incluindo os 120 replays novos, sementes de recuperação, 120 controles do executável anterior e perfil GUI. Todos passaram em integrity_check e chaves estrangeiras; zero órfãos, vetores com versão velha ou grupos de chave ativa duplicada. Dimensões vetoriais atuais 768. O perfil GUI tem 7 tombstones e 66 vetores; filas/leases e contagens por estado estão disponíveis por banco. Contar linhas da FTS externa não é apresentado como uma verificação integral do conteúdo FTS; recuperação real e regressões exercitaram o índice.

As fontes USER dos dois turnos inéditos retidos permaneceram done: a renovação anexou proveniência à preferência existente; a correção foi revista, tornou-se 56 GB e foi recuperada em sessão nova. Zero perdas demonstradas exclusivamente por retenção do verificador nos casos observados. **Não há prova de reaprendizado com tombstone no cenário inédito de classes:** o comando de esquecer não criou tombstone e a preferência original permaneceu ativa. Isso é uma falha do cenário de esquecimento, não renovação bem-sucedida. [SQLite](evidence/phase22d/final-v1/persistence-final.json), [fontes retidas](evidence/phase22d/final-v1/retained-source-audit.json).

O perfil completo termina com 68 registros ativos e 66 vetores atuais. Os dois sem vetor são exclusivamente fixtures nativos de ordem de fontes (desenvolvimento/final), sem vínculo de conversa GUI para rotação automática de indexação; não são omitidos da auditoria nem foram reindexados após o congelamento. A semente C não tem lacuna. Há zero jobs queued/running/retry e zero leases pendentes no snapshot final. SQLite/FTS/btrees, tamanho lógico e arquivos main/WAL/SHM são medidos separadamente; main sozinho não representa todo o banco com WAL. [Tamanhos e lacunas](evidence/phase22d/final-v1/index-size-audit.json).

## C — Recuperação inédita

31 consultas, sementes nativas com revisão QA quando exigida e vetores Nomic reais. Essa etapa isola ranking/filtros da extração Gemma. Contexto inicialmente injetado é separado de consultas posteriores do verificador.

| Modo | TP / FP / perdas | Precisão / recall |
|---|---:|---:|
| Lexical, sem vetor de consulta | 24 / 0 / 2 | 100% / 92,31% |
| Nomic, peso lexical zero | 20 / 1 / 6 | 95,24% / 76,92% |
| Híbrido | 24 / 1 / 2 | 96% / 92,31% |

Zero injeções críticas de ator/namespace/projeto ou excluídas como atuais nesse conjunto. O híbrido ainda injeta linguagem em vez do assunto pedido em `dr-007`. Em `dr-027`, o filtro de irmão também elimina o banco legítimo do usuário numa comparação: **1 perda entre 16 consultas lexicais obrigatórias**, logo fallback lexical não aprovado. Abstenção conservadora não conta como recuperação útil completa. [Métricas/referências](evidence/phase22d/final-v1/retrieval.json), [rastros originais das falhas](evidence/phase22d/final-v1/retrieval-failure-traces.json).

Separadamente, Nomic realmente encerrado por identidade, pergunta nova e sem cache: o fato conhecido de 48 GB foi injetado e a GUI entregou **48 GB.** Zero chamadas de embedding bem-sucedidas nesse intervalo. Um único reinício do mesmo Nomic, mesma pergunta em nova sessão, uma chamada real de embedding e a mesma resposta; canônico inalterado. Falhas GET de attestation não são registradas no log JSON de erros POST, então não se inventa um status HTTP observado. [Parada/recuperação](evidence/phase22d/final-v1/fallback-real.json), [payload real](evidence/phase22d/final-v1/fallback-forwarded-proof.json). É regressão conhecida; não compõe o denominador linguístico inédito.

## D — Contexto conhecido para resposta independente

O adapter/verificador de produção, referências nativas validadas e Gemma/Nomic reais executaram **16 dos 21 casos**; 15 respostas entregues, uma retida. As 20 afirmações entregues foram sustentadas. A preferência excluída foi falsamente apresentada como disponível nas duas tentativas brutas, ambas retidas.

Antes de `da-017`, o avaliador congelado procurou Rust no estado obsolete; o armazenamento nativo mantém esse registro active e restringe sua elegibilidade corrente pelo lifecycle cancelled. A seleção falhou antes de gerar. É defeito de pressuposto do fixture/avaliador, sem prova de falha de escrita do produto. Não foi corrigido após ver os resultados; cinco casos ficaram sem execução.

- Sobre o plano completo: disponibilidade 15/21 = **71,43%**; completude 20/27 = **74,07%**; slots não executados contam zero.
- Entre executados, medida suplementar: disponibilidade 15/16 = 93,75%, completude 20/21 = 95,24%.
- Fidelidade entregue 20/20, zero críticas entregues neste subconjunto; **não certifica os cinco casos ausentes nem a etapa completa**.

[Execução parcial](evidence/phase22d/final-v1/answers-partial.json), [revisão literal e denominadores](evidence/phase22d/final-v1/answer-claim-ledger.json). Referências conhecidas isolam ranking, sem ignorar gates nativos de ator, escopo, temporalidade ou orçamento. Não há executor/publicação de aprendizagem nesse leitor CLI e não são fornecidas observações locais de ambiente como se fossem da GUI.

## E — Ciclo completo inédito na aplicação

44 turnos reais em Embauba, Peroba e Bromélia, mesma identidade/política/SQLite, Chat/Agent/Code, 21 fontes e 23 sondas; sondas em novas conversas. Mensagens reais do composer passam pelos hooks e host nativo, revisão no painel, indexação e leitura. Controle DOM do WebView próprio, sem mouse, teclado ou captura da tela do usuário.

| Métrica literal | Resultado |
|---|---:|
| Afirmações entregues sustentadas | 35/37 = **94,59%** |
| Críticas entregues | **1** |
| Disponibilidade útil | 37/44 = **84,09%** |
| Completude | 41/53 = **77,36%** |
| Turnos retidos | 2 |

A afirmação crítica diz que o usuário ainda não forneceu os detalhes do banco que planejou, apesar do SQLite explicitamente informado e armazenado. Outra resposta aceita nega registros do catálogo/tecnologias sem delimitar a ausência à recuperação. Ausências explicitamente limitadas aos registros disponíveis ou informações recuperadas mantêm sua classificação literal anterior: podem ser sustentadas e ainda falhar em utilidade.

RAM 56/24, comparação dos dois equipamentos, café antes/agora, bebida favorita, objetivo concluído, GPU ausente e irmão sem dados funcionaram. Perdas incluem Rust escolhido antes da implementação TypeScript, tema do catálogo, comparação usuário/irmão, composto catálogo/tecnologias e PostgreSQL em pergunta composta do segundo projeto. O estado cancelado e a implementação TypeScript **antes** de cancelar foram preservados.

Na resposta de RAM corrigida, o delta correto de 24 GB foi retido: falso bloqueio preservado, mesmo com aprendizagem posterior correta. No pedido de renovação, alegações brutas de guardei foram retidas sem perder a fonte. Agent também executou file_list/file_search somente na pasta QA vazia; a conclusão indevida de que essa pasta provaria linguagem indefinida foi barrada e regenerada. Não se aprovou nenhuma edição de arquivo.

Reparo localizado real havia sido observado no desenvolvimento: retirou uma promessa sem recibo e preservou a preferência. Outro reparo removeu uma negativa correta; não se apresenta o mecanismo como inteiramente aprovado. Nenhuma nota de reflexão foi observada nesta rodada final; zero notas sem verificação **não** é um novo teste intensivo desse caminho. O botão de cópia não foi novamente exercitado na avaliação final; regressões anteriores permanecem.

[Todos os turnos e revisão literal independente](evidence/phase22d/final-v1/gui-literal-review.json). Perguntas, conselho geral, intenção futura e persona do ASSISTANT ficam fora do denominador de fatos USER; pedidos de confirmação redundantes e antropomorfismo continuam defeitos de naturalidade. A revisão não usa o veredicto do aplicativo como verdade.

## Regressões longitudinais originais

Ordem exata **20 → 50 → 100**, mesmo `phase22c-final-historical-series`, projetos **candeia → candeia2 → candeia**, mesmo perfil persistente de QA. Sem copiar banco operacional e sem novo escopo vazio na série 100. 170 turnos originais e 28 sondas, corpora e hashes originais intactos.

| Série | Recuperação estrita P/R | Sondas entregues | Fidelidade literal | Críticas entregues | Disponibilidade/completude de todos os turnos |
|---|---:|---:|---:|---:|---:|
| 20 | 100,00% / 100,00% | 9/10 | 11/13 | 0 | 93,33% / 87,50% |
| 50 | 100,00% / 100,00% | 8/8 | 14/14 | 0 | 100,00% / 100,00% |
| 100 | 90,91% / 100,00% | 10/10 | 25/26 | 1 | 98,18% / 98,21% |

A série 20 reteve a extrapolação está sendo desenvolvido que antes passava, mas não entregou a descrição/plano útil. Duas frases de desempenho baseadas só em RAM passaram e contam como não sustentadas. A série 50 não reteve turnos e preservou seus 14 fatos. Na série 100, uma regeneração Gemma real de 3,199 s retirou a execução indevida e preservou descrição, escolha Rust e plano PostgreSQL; isso é regeneração, não projeção local. A fonte Rust retida no turno 21 foi aprendida e recuperada. O turno 92, porém, entregou Registrado antes de criar a candidata: o limite superior conservador da entrega antecede a criação nativa em mais de quatro segundos. A revisão posterior não atesta retroativamente uma gravação; conta como crítica entregue. A exclusão de café dessa série gerou dois tombstones reais e a renovação seguinte foi revista; não deve ser confundida com o esquecimento inédito de classes que falhou. As expressões antigas são mantidas apenas como diagnóstico; regex não define verdade semântica. [20](evidence/phase22d/final-v1/historical-20-review.json), [50](evidence/phase22d/final-v1/historical-50-review.json), [100](evidence/phase22d/final-v1/historical-100-review.json), [relação e momento da gravação](evidence/phase22d/final-v1/historical-100-relation-proof.json).

Nas 255 tentativas brutas únicas de resposta da GUI, inéditas e históricas, a revisão contou 115 afirmações: 101 sustentadas, nove críticas e cinco não críticas indevidas. Sete críticas foram impedidas; duas chegaram à entrega (nunca fornecido e Registrado prematuro). Os denominadores primários de cada conjunto permanecem separados. [Todas as tentativas e custos de intervenção](evidence/phase22d/final-v1/all-gui-raw-claim-review.json).

## Falhas reais e limites de robustez

Seis protocolos congelados: cancelamento antes de publicar em três modos, fonte inexistente, ASSISTANT como USER, retry/idempotência, ordem de correção versus fonte antiga, cancelamento de lease com resposta Gemma real tardia e reinício real do host durante extração. Resultado: **6/6**. A recuperação por reinício exigiu nova tentativa, não duplicou quatro fatos e rejeitou a completion da lease antiga. Entradas desses testes são fixtures nativos preparados; isso não é apresentado como envio pelo composer em todos os casos. [Provas sem tokens/leases](evidence/phase22d/final-v1/native-faults.json).

Teste suplementar de infraestrutura após o holdout, reutilizando a fonte literal já congelada, sem alterar avaliador/corpus: broker próprio realmente desligado, falha de transporte observada no composer e um restabelecimento por caso contado. Chat/Agent/Code: **3/3 fontes USER preservadas**, um job done e uma preferência ativa por caso. Nenhuma resposta artificial substitui a falha. É suplemento de protocolo conhecido, não nova generalização linguística. Os formatos de erro diferem: Chat preenche error; Agent escreve Agent error sem esse campo e o observador de caminho feliz termina; Code exibe HTTP 503 sem finishReason/usage e o observador expira. A resposta de erro não conta como resposta modelo útil. Uma tentativa inicial Agent recuperou a conexão cedo demais por ler DOM antigo: resposta normal excluída da prova de falha; novo escopo usado uma vez. Chat e Code não foram reenviados. Falha inicial do observador externo ao importar URL TypeScript não servida foi preservada; a leitura foi feita por DOM e pelo observador nativo existente, sem alterar produção/congelamento. [Transporte real](evidence/phase22d/final-v1/real-transport.json).

Tombstones, replay, deduplicação, conflitos/correções, migrações, backup/restore, reindexação e concorrência também permanecem na suíte técnica. Não se atribui a cada teste unitário um novo crash real independente de integração. Falha do transporte do modelo não é crash real do RPC Tauri.

## Recursos, custo e encerramento

| Recurso | A: observado na janela final | B: projeção somente |
|---|---:|---:|
| Menor RAM física disponível | 1190.8 MiB | 3238.8 MiB com +2.048 MiB |
| Maior commit do sistema | 22221.3 MiB | Nenhum ganho presumido |
| Menor folga de commit | 10502.0 MiB | Nenhuma alteração presumida |
| VRAM máxima do dispositivo | 9516 MiB | Nenhum ganho presumido |

370 chamadas Gemma e 376 Nomic reais; 758384 tokens de entrada Gemma e 16795 de saída. Contexto **configurado 65.536**, maior prompt observado 6720; isso não é ensaio de saturação. Custos por extração/resposta, avaliação de prompt, geração, CPU dos processos, indexação, verificador e latência nativa estão separados nas evidências. CPU/RAM/commit/VRAM globais incluem aplicativos do usuário; o sampler do host não cobre toda a árvore WebView.

Pagefile e drivers não foram alterados; o pico desde o boot de 8.007 MiB não foi atribuído ao ensaio. B não foi medido e não houve fechamento de ChatGPT para simulá-lo. O broker adapta conclusão real não streaming para SSE: **streaming nativo do modelo não foi validado**. Teste de 200 turnos e repetição de 62.545 tokens não realizados, conforme opcionais/restrição do roteiro. [Recursos e hashes de payload](evidence/phase22d/final-v1/resources-final.json).

135 testes Rust (126 anteriores), 711 TypeScript (687 anteriores), 3 de esquema atômico; typecheck, lint dos alterados e builds de host/bridges/frontend QA aprovados. Nenhuma linha removida dos testes anteriores. Benchmarks históricos de extração/recuperação permaneceram na suíte Rust. Builds pesados com modelos desligados. Pesos completos, launcher e 54 arquivos de suporte do runtime foram novamente conferidos por checksum em leitura após desligar os modelos, sem alocar pesos inteiros em RAM. [Verificações](evidence/phase22d/final-v1/technical-checks.json), [hashes finais](evidence/phase22d/final-v1/final-hashes.json), [runtime completo](evidence/phase22d/final-v1/runtime-bundle-final.json), [executáveis observados](evidence/phase22d/final-v1/executables-final.json).

Processos próprios GUI/Vite/WebView/Gemma/Nomic/broker/sampler encerrados por identidade e portas 15283/15284/15287/15288/15289 fechadas. Nenhum processo de aplicativo existente encerrado. [Encerramento](evidence/phase22d/final-v1/cleanup.json). Instalação operacional somente auditada por leitura; arquivos instalados conservam datas anteriores a esta fase. Não foi coletado baseline byte a byte do perfil operacional antes do ensaio, portanto não se afirma identidade binária do banco vivo; ele não foi aberto nem migrado por esta fase. [Inventário](evidence/phase22d/final-v1/operational-install-readonly.json).

## CI e publicação

Investigação somente por leitura; nenhuma mudança administrativa ou ampliação de permissão, merge ou dispatch repetido. O workflow de memória está ausente de master, a branch padrão (404 conferido), o que impede elegibilidade normal de workflow_dispatch conforme [documentação GitHub](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow?tool=webui). Isso **não comprova** a causa da mensagem anterior HTTP 422 Actions has been disabled for this repository, pois leituras anteriores também mostravam Actions habilitadas. O pipeline remoto não é declarado verde. Estado exato de runs/checks no último HEAD publicado fica na auditoria externa e descrição do PR, sem ciclo de alterar o próprio SHA.

## Pendências para outra rodada de desenvolvimento

Autoria relatada/ficcional antes de qualquer candidata, três paráfrases ainda perdidas, esquecimento inédito sem tombstone, comparação com múltiplos atores, intenção descritiva/composta, histórico de escolha separado da execução, falso nunca-fornecido, garantia de desempenho e falsos bloqueios. Corrigir também o pressuposto de estado do fixture D antes de um **novo** congelamento e um novo conjunto independente. O conjunto desta fase permanece imutável e reprovado. Permanecer no escopo de memória; nenhuma aprovação de autonomia confiável foi concedida.
