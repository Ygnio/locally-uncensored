# Lucille 2.2B — resultado parcial, sem aprovação operacional

A camada de memória foi aprimorada e publicada no [PR em rascunho #10](https://github.com/Ygnio/locally-uncensored/pull/10), empilhado sobre a Fase 2.2. **A fase não está concluída:** a avaliação inédita final atingiu **93,22% de fidelidade, com 3 violações críticas**, abaixo de ≥95% e zero críticas. Não houve merge, release ou instalação operacional. Não foi iniciada outra funcionalidade.

## Código, congelamento e reprodução

Base remota conferida antes do trabalho: `c1840255292724cc07cb3892f964619e39b5584e`. Branch: `feature/lucille-phase22b-answer-fidelity`, contra `feature/lucille-phase22-autonomous-memory`. O PR #9 permanece sem merge.

Código final de produção: `03ca5fece02b06c8f0a1e286c7ee784175f28e3d`; backend nativo compilado a partir de `2c38b650`, sem alteração nativa posterior. Os nove conjuntos v3 foram publicados em `0d9b86b2`, depois do encerramento do desenvolvimento, e o congelamento em `c753ea1c`, antes da execução. Hash do [freeze v3](../crates/lucille-memory/tests/fixtures/phase22b/freeze-v3.json): `2c1736d668e5886f2d86d2793642b8f8860bafabc6dc4dfa2b74fe06d3e94ea5`.

O congelamento contém 4.150 fontes, 30 corpora, hashes de executáveis e frontend, modelos/runtime, identidade sintética, política, schema 5, representação/índice 5 e protocolo atômico 3. Os controladores conferiram essas impressões antes e depois dos cenários. Não se alteraram fontes, prompts, pesos, política, ranking, limites ou gold durante a avaliação. Os testes antigos permaneceram intactos e foram tratados como regressão conhecida.

O [diagnóstico e desenho](lucille-phase22b-design.md) descreve as dez falhas históricas reproduzidas antes das alterações. Os 54 hashes de fonte históricos permaneceram iguais durante esse diagnóstico. As rodadas negativas [v1](evidence/phase22b/v1/manifest.json) e [v2](evidence/phase22b/v2/manifest.json) foram preservadas, sem reclassificá-las como aprovadas. A v1 teve fidelidade 81,82% e cinco críticas; a v2, 72,58% e quatro críticas. A preparação da recuperação v2 falhou na extração da fonte e não recebeu métricas fictícias de consultas.

## Implementação entregue

- Fatos contextuais tipados preservam entidade, propriedade, valor, modalidade, validade atual/histórica, escopo, revisão, versão e evidência. Plano, escolha, execução, conclusão, cancelamento e ausência de registro permanecem distintos. A falta de resultados de busca não prova ausência de decisão.
- O verificador lê novamente a autoridade canônica nativa e permite no máximo uma regeneração real. Guarda texto bruto, violações, confirmação adicional, texto final, tokens e custo. Não usa o próprio modelo como juiz e não cria respostas substitutas programadas.
- Chat, Agent e Code seguram o texto até o veredito. A fonte USER original é reservada antes da geração, publicada depois da resposta durável e revogada em cancelamento, expiração ou reinício. Resposta do assistente, conteúdo expandido e fontes não elegíveis não se tornam autoridade de aprendizagem.
- Pedidos limitados de esquecimento geram comprovante transacional, cancelam trabalho incompatível e preservam tombstones. O reaprendizado explícito exige revisão quando necessário; replay de fonte antiga não substitui estado posterior. Correções de RAM só vinculam equipamento quando a capacidade anterior o identifica de forma única.
- A recuperação separa propriedades e equipamentos, cobre paráfrases de café e perguntas indiretas, mantém preferências distintas e admite erros limitados de vocabulário. Os limites globais não foram reduzidos. Vetores físicos obsoletos são recolhidos sem apagar o histórico canônico permitido.
- A identidade permanece no perfil explícito; não depende de alias do modelo. Schema e migrações operacionais foram preservados. Os cinco conceitos seguros de preferência foram ampliados sem promoção automática de hardware, projetos, identidade, permissões ou dados sensíveis.

## Extração inédita v3

64 fontes, 48 fatos gold, 33 fontes A, 15 B e 16 C. Uma resposta real do Gemma por fonte foi reproduzida inalterada nos modos nativos; regras não consomem a saída do LLM. A precisão abaixo é da extração canônica após validação independente, não uma alegação de que toda proposta bruta do modelo era correta.

| Modo | TP | FP | Perdidos | Precisão | Recall | Promoção automática correta |
|---|---:|---:|---:|---:|---:|---:|
| rules | 44 | 0 | 4 | 100,00% | 91,67% | 29/29 |
| llm | 42 | 0 | 6 | 100,00% | 87,50% | 29/29 |
| hybrid | 44 | 0 | 4 | 100,00% | 91,67% | 29/29 |

Houve 62 propostas brutas e zero saídas JSON malformadas. Cada modo promoveu 29 casos corretos, sem promoção crítica ou vazamento de escopo. São as mesmas fontes repetidas entre modos, não 87 fontes independentes. Critérios globais: precisão ≥95%, recall ≥80%, promoção ≥99% e zero críticas. A amostra é limitada; não demonstra precisão populacional de 100%.

As cinco categorias seguras, no modo LLM com a resposta real e validação nativa, tiveram:

| Categoria | TP | FP | Perdidos | Auto corretas | Auto incorretas |
|---|---:|---:|---:|---:|---:|
| Criação de personagens | 4 | 0 | 1 | 4 | 0 |
| Escolha de classes | 4 | 0 | 1 | 4 | 0 |
| Rejeição de interface carregada | 5 | 0 | 0 | 5 | 0 |
| Bebida favorita | 4 | 0 | 1 | 4 | 0 |
| Respostas objetivas | 5 | 0 | 0 | 5 | 0 |

As categorias negativas, sensíveis, de autoridade, terceiros, citações e hipótese/condição estão discriminadas nas evidências; não receberam gravação crítica ou promoção indevida. Zero fatos propostos/persistidos não é denominado precisão populacional de 100%.

## Recuperação inédita v3

22 eventos de preparação, 37 consultas e 38 fatos esperados, com Nomic real e processos nativos novos para as consultas. A preparação dos fatos revisados usa regras nativas e a revisão correspondente; a qualidade da extração pelo Gemma foi medida separadamente acima.

| Configuração | TP | FP | Perdidos | Precisão | Recall |
|---|---:|---:|---:|---:|---:|
| fts5 | 31 | 0 | 7 | 100,00% | 81,58% |
| nomic | 26 | 0 | 12 | 100,00% | 68,42% |
| hybrid | 36 | 0 | 2 | 100,00% | 94,74% |

O híbrido de produção passa no limite global de 90%/80%, com zero referências inseguras, extras ou de outro escopo. Equipamentos: 10/10; café: 5/5; perguntas indiretas: 3/3. **Histórico de projeto: 0/2 fatos em uma pergunta composta**, omissão que permanece pendente apesar do agregado. O estado cancelado tinha cosseno 0,7033, acima do limiar, mas foi excluído pelo filtro agregado de temporalidade/entidade/intenção. O [diagnóstico preservado](evidence/phase22b/v3/historical-retrieval-diagnosis.json) não atribui essa falha apenas ao embedding. O fallback FTS5 passa no agregado. O Nomic isolado não passa no recall e não está aprovado como configuração isolada. Não se alterou o limiar para melhorar o resultado.

## Respostas reais na aplicação completa

Seis cenários inéditos, incluindo 32 mensagens de conversa e 14 perguntas em novas sessões. Foram avaliados 65 espaços de resposta final, 71 tentativas dos relatórios finais e uma rodada adicional de narração antes de ferramenta no Agent: **72 tentativas reais ao considerar todos os relatórios de fidelidade**. A [revisão adicional](evidence/phase22b/v3/tool-round-literal-review.json) cobre essa rodada com texto/hash exatos; ela não acrescenta afirmação de memória ao denominador. O avaliador congelado cobre o último relatório de cada resposta final, e essa limitação é explicitada.

Revisão literal independente das respostas finais: **55/59 afirmações sustentadas, 93,22%; três críticas**. Bruto do relatório final: 69/77, 89,61%, cinco críticas; a narração adicional não contém afirmações de memória. Houve 61/65 respostas entregues, quatro retidas, duas regeneradas e 59 aceitas sem regeneração. Retenção não é acerto: foram perdidos seis dos 70 elementos úteis esperados. A mesma pontuação foi reproduzida sobre as cópias públicas das evidências.

| Categoria | Afirmações sustentadas | Críticas | Entregues / pedidos | Elementos úteis perdidos |
|---|---:|---:|---:|---:|
| longitudinal_nonprobe | 26/28 | 1 | 38/40 | 2 |
| plan_vs_execution | 2/2 | 0 | 1/1 | 0 |
| multilingual_question | 2/2 | 0 | 2/2 | 0 |
| indirect_question | 5/5 | 0 | 2/3 | 2 |
| different_projects | 1/1 | 0 | 1/1 | 0 |
| decision_vs_completion | 1/2 | 1 | 2/2 | 0 |
| ram_correction | 1/1 | 0 | 1/1 | 0 |
| different_equipment | 2/2 | 0 | 1/1 | 0 |
| current_vs_historical | 3/3 | 0 | 2/3 | 1 |
| corrected_preference | 6/6 | 0 | 4/4 | 0 |
| completed_goal | 1/1 | 0 | 1/1 | 0 |
| deleted_information | 1/1 | 0 | 1/1 | 0 |
| no_memory | 2/2 | 0 | 2/2 | 0 |
| cancelled_project | 0/1 | 1 | 1/1 | 1 |
| absence_vs_negative_decision | 2/2 | 0 | 2/2 | 0 |

As quatro afirmações finais sem suporte foram:

1. Uma decisão de usar TypeScript foi seguida por “o projeto ainda está em desenvolvimento”, sem fonte que comprove início da execução. Crítica.
2. Um pedido para voltar a memorizar uma preferência foi interpretado como “voltou a gostar”. A exclusão de um registro não comprova mudança de gosto. Crítica.
3. O Agent usou uma chamada real `file_list` e inferiu da pasta de QA vazia que o projeto lembrado não tinha arquivos, omitindo o cancelamento canônico. A pasta vazia é resultado real da ferramenta; sua equivalência ao estado do projeto não é comprovada. Crítica, além da ferramenta desnecessária.
4. Depois da regeneração, uma resposta ainda atribuiu capacidade excelente para executar ambiente e banco simultaneamente a partir apenas da quantidade de RAM. Extrapolação de desempenho sem evidência, não crítica.

O verificador também reteve informação correta nas tentativas seguintes: decisão de troca de banco, confirmação idempotente de esquecimento, lembrança indireta do projeto e linguagem histórica do projeto cancelado. Esses falsos positivos continuam pendentes. Duas perguntas em inglês receberam resposta em português; a não conformidade de idioma é separada da verdade factual.

A observação DOM não registrou pintura com veredito ausente. A observação do Store encontrou uma cópia posterior da nota intermediária do Agent sem os metadados de fidelidade: o mesmo texto já havia sido verificado no primeiro relatório, antes da chamada de ferramenta. Esse registro foi preservado e correlacionado, não removido da evidência. A segurança geral de fluxos arbitrários de ferramentas não é declarada aprovada. O broker adapta conclusão não streaming para SSE; streaming nativo continua sem aprovação.

## Regressões históricas obrigatórias

Estes são os conjuntos v3 conhecidos da Fase 2.2, não avaliação inédita. Recuperação inicial do payload, confirmações adicionais do verificador e texto realmente exibido foram contabilizados separadamente. Os padrões originais não foram relaxados. A série válida usa o mesmo escopo para 20, 50 e 100, como no protocolo original: o projeto candeia2 do cenário 50 é separado do candeia, retomado no 100. Uma tentativa inicial isolou indevidamente os escopos; seu 100 parou após 61 mensagens sem o histórico de hardware necessário. Esse erro meu de preparação, a tentativa parcial e a repetição correta estão preservados na [errata de orquestração](evidence/phase22b/v3/regression-orchestration-erratum.json); a tentativa inválida não é apresentada como conclusão de 100 mensagens nem misturada ao escore da série correta.

| Mensagens | Fatos recuperados / esperados | Extras | Referências críticas | Respostas entregues / probes | Padrão original conservador |
|---|---:|---:|---:|---:|---:|
| 20 | 10/10 | 0 | 0 | 8/10 | 8/10 |
| 50 | 8/8 | 0 | 0 | 7/8 | 6/8 |
| 100 | 10/10 | 1 | 0 | 9/10 | 9/10 |

Na tentativa isolada adicional de 50, a resposta inglesa “No relevant record is available...” foi uma ausência delimitada correta, embora não corresponda à expressão original de comparação. O escore automático original foi preservado e o caso recebe revisão literal explícita, sem mudar gold ou expressão. Bloqueios de respostas úteis continuam contados como indisponibilidade. O 100 inclui um `games_turn_strategy` ativo e relacionado à pergunta de RPG, além de `games_tactics`; o gold antigo lista somente este último. O FP estrito foi preservado como 1, sem editar o conjunto. O [complemento de revisão](evidence/phase22b/v3/historical-regression-literal-notes.json) registra também o falso bloqueio do objetivo atual no 20 e a cautela com “está avançando” no 50: passar em uma expressão antiga não comprova fidelidade completa. O teste opcional de 200 mensagens não foi executado; o ensaio histórico de 62.545 tokens com Gemma isolado não foi repetido. O contexto configurado permaneceu em 65.536.

## Falhas controladas e persistência

Os [testes reais de desenvolvimento](evidence/phase22b/development-v3/manifest.json), no backend final, demonstraram:

- Nomic desligado: resposta por busca textual, um vetor pendente; após religar, um vetor atual, sem duplicação ou pendência.
- Cancelamento antes de publicar a fonte: nenhuma memória ou autoridade posterior; publicação tardia recusada.
- Cancelamento com lease de extração real em andamento: resposta real do Gemma apresentada à lease revogada foi recusada; nenhuma memória ou proveniência tardia.
- Reinício da janela durante extração, antes do commit: retomada automática produziu quatro memórias, quatro proveniências e quatro vetores, sem duplicação ou pendências. Não foi teste de interrupção dentro do fsync do SQLite.

A conversa inédita teve também um job terminal `extractor_failed`, com três tentativas, na repetição “O projeto continua sendo um catálogo de minerais”. A descrição original permaneceu ativa, mas essa nova fonte não foi aprendida/reforçada. As [respostas reais e a falha](evidence/phase22b/v3/gui-extraction-failure.json) foram preservadas; não se confundem com o sucesso do conjunto isolado de 64 fontes. Há outros três jobs falhos de rodadas anteriores no banco sintético global, também identificados nas evidências. Não há lease órfã nesses trabalhos terminais. O replay na conversa inédita manteve o estado canônico inalterado. Os detalhes finais preservam os dois equipamentos, as trilhas separadas de linguagem planejada/uso atual e a decisão de banco. A exclusão do tema permaneceu efetiva. Reaprendizado e repetições não criaram memórias ativas duplicadas para as cinco propriedades seguras.

Auditoria final somente leitura do banco sintético: integridade `[['ok']]`, 0 violações de chave estrangeira; schema 5; 0 jobs não terminais; 0 jobs terminais com lease remanescente; 0 vetores obsoletos/versão incorreta; 0 em espaços inativos; 0 expirados. Histórico: 386 eventos, 132 revisões, 8 tombstones e 192 proveniências. Os totais abrangem outros escopos sintéticos de desenvolvimento, não somente a v3.

## Testes locais e recursos

Passaram **122 testes Rust, 666 TypeScript e três de esquema atômico**, sem falhas ou skips/ignores; typecheck, lint dos arquivos alterados e builds da aplicação nativa, bridge e interface isolada passaram. Os 89 Rust, 563 TypeScript e os testes nativos de paridade/integração anteriores foram preservados como subconjunto. Isso não substitui os gates reais reprovados nem comprova CI remoto.

Gemma 4 12B Q4_K_M, contexto 65.536, GPU; Nomic v1.5 F16, CPU, dimensão 768 e contexto 2.048; runtime fixado e os mesmos pesos locais. Nenhum download novo nesta fase. A inferência foi serializada. Builds pesados ocorreram com os modelos desligados.

Cenário A real, abrangendo a rodada final e as regressões: mínimo disponível 1809 MiB; mínimo de margem de commit 11.20 GiB; pico de VRAM observada 9395 MiB; pagefile em uso atingiu 1145 MiB. O pico histórico global de 8.007 MiB já existia e não é atribuído a esta rodada. A medição de processo da GUI cobre só o host nativo; os totais do sistema incluem WebView e aplicativos do usuário.

Cenário B é **somente projeção**: +2.048 MiB físicos disponíveis, mínimo projetado 3857 MiB, sem melhorar artificialmente a margem de commit. Nenhum programa existente foi fechado para obtê-lo.

Na GUI inédita, geração observada: mediana 2.92s, p95 6.24s, máximo 12.49s. Foram 111 chamadas Gemma na janela, incluindo extração e seleção de ferramenta, com 122,312 tokens de entrada e 4,214 de saída; 102 chamadas Nomic. O maior prompt observado nessa janela foi 3,828 tokens; não se afirma saturação dos 65.536. Custos brutos, regeneração, confirmação, indexação e crescimento físico estão nos [dados de execução](evidence/phase22b/v3/runtime-new-corpus.json) e [recursos/estado nativo](evidence/phase22b/v3/resources-and-native-state.json).

A única regeneração permitida por resposta acrescentou, no total dos relatórios finais, 14.43s, 8,566 tokens de entrada e 121 de saída; esses custos incluem tentativas cuja resposta acabou retida. Verificação: mediana 0.90ms, p95 2.20ms; leitura nativa fresca: mediana 7.60ms, p95 19.50ms. A rodada adicional anterior à ferramenta tem seus próprios tempos no suplemento. O índice físico final ocupou 1346.2 KiB em vetores JSON, e o banco sintético, 2192.0 KiB em páginas principais; WAL/SHM máximos estão nas medições de recursos.

## Preservação, publicação e pendências

Foram usados somente checkout, executáveis, banco, perfis, WebView e portas próprios de QA. Aplicação em Program Files, perfis/conversas/identidade/SQLite operacionais, launchers e pesos originais, drivers, pagefile e portas operacionais não foram alterados. Nenhum aplicativo existente foi encerrado. A finalização parou somente processos próprios com identidade verificada. A GUI foi conduzida por observador DOM na própria janela, sem controle do mouse/teclado do sistema; isso não comprova correção da captura global Remote do Codex.

As [evidências sintéticas v3](evidence/phase22b/v3/manifest.json) preservam textos, hashes, saídas, intervenções e falhas. As cópias públicas retiram somente mensagens SYSTEM com contexto privado; USER/assistant avaliados, IDs e hashes das respostas permanecem. Não foram publicados pesos, bancos, conversas reais, perfis, credenciais ou logs privados. A [lista de publicação](evidence/phase22b/v3/publication-inputs.json) registra arquivos e commits anteriores ao fechamento; o HEAD final exato e a auditoria remota são registrados no corpo do PR e no relatório local após o último commit, evitando alterar o SHA auditado.

O HTTP 422 de Actions permanece separado da qualidade da memória. A auditoria delimitada do HEAD final é registrada no corpo do PR e no relatório local, com uma única tentativa e leituras limitadas, sem alteração administrativa. **CI remoto não está declarado verde.** A ausência de execução comprovada não será substituída pelos testes locais.

Permanecem para uma nova rodada, com novo congelamento e novos casos: impedir extrapolações temporais/estado do projeto e mudança de gosto, corrigir os falsos bloqueios, recuperar o histórico de projeto, evitar ferramentas para perguntas só de memória, conservar metadados das notas do Agent e respeitar o idioma pedido. Não avançar para outras funcionalidades nem instalar esta versão operacionalmente enquanto esses critérios falharem.
