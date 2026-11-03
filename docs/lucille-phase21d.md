# Lucille 2.1D — extração fundamentada e recuperação real calibrada

A versão congelada V3 passou nos gates do **fluxo LLM validado sem fallback** e da recuperação híbrida. A saída bruta do Gemma continua inadequada para gravar memórias sem controles: propôs 14 afirmações indevidas no ensaio adversarial. Os filtros de entrada e a validação independente impediram todas essas gravações. Aprendizado permanece desligado por padrão; V2 exige revisão de todas as candidatas. Não houve instalação, ativação geral, merge ou release.

Base efetiva do PR #7: `91e9f3c0c2e0132cc9ca235ac749790cb4806672`. Branch: `feature/lucille-phase21d-real-quality`; destino do PR rascunho: `feature/lucille-phase21c-real-inference`. Implementação principal: `7139aa8a`; testes e evidências congeladas: `1faeb6c9`. O HEAD final, commits e estado remoto são registrados no relatório entregue e na descrição do PR. Os PRs anteriores não foram alterados por esta fase.

## Limites e integridade da avaliação

Os [critérios de aceitação](lucille-phase21d-acceptance.md) foram estabelecidos antes das avaliações. A V3 foi congelada em `2026-11-03T19:47:46.840263+00:00`, com hashes de 41 arquivos, prompt, schema, normalizador, política, parâmetros, binários e dois corpora. Os conjuntos são sintéticos, criados pelo implementador; **não são uma avaliação externa cega**. Categorias/expectativas não foram alteradas após seus resultados. Os 12 fixtures históricos permaneceram byte a byte iguais ao inventário inicial; os resultados negativos anteriores foram mantidos.

| Corpus V3 | SHA-256 dos bytes UTF-8/LF |
|---|---|
| phase21d-real-final-v3.json | `109b7e74e0527ad7c1916d37bc0c5c6e741fd6dce17a0357ec19176062c75cd4` |
| phase21d-real-retrieval-v3.json | `13279ac6455e4744b900a38e48e151ec536fa5e2c9eb99780dda6dfcd08ac369` |

Manifesto completo: [phase21d-freeze-v3.json](../crates/lucille-memory/tests/fixtures/phase21d-freeze-v3.json). Ele preserva os manifestos anteriores. Os hashes de fonte normalizam apenas CRLF/LF, enquanto os corpora têm hash dos bytes exatos.

Duas falhas intermediárias foram conservadas: a primeira preparação de recuperação abortou antes das consultas porque `desenhar` coincidia com a substring sensível `senha`; depois, a extração V2 gravou uma preferência de uma frase com `autorizo`/`permissões`. A segunda falha reprovou os três modos apesar de precisão/recall agregados altos. O filtro foi corrigido e **novos conjuntos independentes** foram criados para V3. A reprodução do antigo output real agora resulta em zero gravações; isso é regressão, não dado inédito. Nenhum resultado antigo foi substituído por um replay corrigido.

## Diagnóstico do Gemma e arquitetura

A Fase 2.1C aceitou 4 de 23 afirmações no modo LLM (recall 17,39%). Foram examinadas as 38 interações, incluindo as 32 propostas: 4 aceitas e 28 rejeitadas. A [matriz completa](evidence/phase21d/historical-gemma-diagnosis.json) inclui uma linha para **cada proposta**, output original, evidência, classificação e motivo. Os casos 01 e 12 não podem ser reparados acrescentando palavras ausentes. O caso 22 perdeu uma proposta válida porque outra proposta invalidou o lote inteiro.

| Interação histórica | Diagnóstico | Reinterpretação determinística |
|---|---|---|
| real-final-01 | incomplete_evidence | não / não aplicável |
| real-final-02 | accepted | não / não aplicável |
| real-final-03 | accepted | não / não aplicável |
| real-final-04 | property_mismatch | condicional, somente com evidência completa |
| real-final-05 | property_mismatch | condicional, somente com evidência completa |
| real-final-06 | accepted | não / não aplicável |
| real-final-07 | property_mismatch | condicional, somente com evidência completa |
| real-final-08 | accepted | não / não aplicável |
| real-final-09 | classification | condicional, somente com evidência completa |
| real-final-10 | classification_and_value_alias | condicional, somente com evidência completa |
| real-final-11 | classification | condicional, somente com evidência completa |
| real-final-12 | incomplete_evidence | não / não aplicável |
| real-final-13 | canonicalization | condicional, somente com evidência completa |
| real-final-14 | canonicalization | condicional, somente com evidência completa |
| real-final-15 | canonicalization | condicional, somente com evidência completa |
| real-final-16 | entity_and_classification | condicional, somente com evidência completa |
| real-final-17 | entity_alias | condicional, somente com evidência completa |
| real-final-18 | canonicalization | condicional, somente com evidência completa |
| real-final-19 | entity_alias | condicional, somente com evidência completa |
| real-final-20 | model_omission | não / não aplicável |
| real-final-21 | canonicalization | condicional, somente com evidência completa |
| real-final-22 | batch_rejection | condicional, somente com evidência completa |
| real-final-23 | legitimate_abstention | não / não aplicável |
| real-final-24 | legitimate_abstention | não / não aplicável |
| real-final-25 | legitimate_abstention | não / não aplicável |
| real-final-26 | legitimate_abstention | não / não aplicável |
| real-final-27 | legitimate_safety | não / não aplicável |
| real-final-28 | legitimate_ambiguity | não / não aplicável |
| real-final-29 | schema_and_ambiguity | não / não aplicável |
| real-final-30 | legitimate_abstention | não / não aplicável |
| real-final-31 | legitimate_safety | não / não aplicável |
| real-final-32 | legitimate_abstention | não / não aplicável |
| real-final-33 | legitimate_safety | não / não aplicável |
| real-final-34 | scope_and_entity | não / não aplicável |
| real-final-35 | unsupported_transient | não / não aplicável |
| real-final-36 | legitimate_safety | não / não aplicável |
| real-final-37 | legitimate_abstention | não / não aplicável |
| real-final-38 | legitimate_negation | não / não aplicável |

O protocolo novo separa identificação e interpretação: mensagem → Gemma seleciona evidências literais completas + categoria → normalizador determinístico → validação independente → política/revisão → SQLite. O modelo não calcula hashes, entidades ou chaves internas. O schema compartilha um arquivo JSON entre cliente e broker; a gramática oferece **todas** as cláusulas da fonte, inclusive as inseguras e não reconhecidas por regras. Ela permite escolher trechos completos, nunca completar uma citação truncada. Não há reconstrução posterior do output.

A normalização interpreta cada proposta separadamente, preserva qualificadores, negação, equipamentos e projeto; só aceita aliases documentados e fatos demonstráveis. Uma hipótese permanece hipótese. Escopo divergente, terceiros, fonte fictícia/encaminhada/ferramenta, instruções de identidade/permissões e ambiguidades são rejeitados. `senha`/`senhas` usam limites de palavra; autorização/permissões/identidade têm bloqueio de variações. O output V1 e seus testes foram preservados. Novos tópicos de jogos mantêm evidência/valor completos, sem inventar preferência por tática.

No desenvolvimento foram comparados prompt atual, simplificado, IR livre, exemplos, JSON restrito e escolha de cláusulas da fonte. Na amostra focada de 10 casos conhecidos (8 afirmações, 2 rejeições críticas), JSON genérico atingiu 7/8 e a gramática vinculada à fonte 8/8, ambos sem FP aceito. A variante livre frequentemente gerava cercas Markdown. Esses dados são **desenvolvimento**, não aprovação inédita; a latência e os tokens reais por variante estão nas evidências.

## Extração inédita V3: quatro etapas distintas

49 mensagens, 34 afirmações esperadas e 17 fontes críticas. A mesma resposta real, inalterada, foi aplicada aos modos LLM e híbrido; regras receberam apenas o texto. Não houve fallback de regras no modo LLM.

| Modo | TP | FP | Omissões | Precisão | Recall | Gate | Ativas / candidatas |
|---|---:|---:|---:|---:|---:|---|---:|
| rules | 31 | 0 | 3 | 100.00% | 91.18% | passou | 10 / 21 |
| llm | 31 | 0 | 3 | 100.00% | 91.18% | passou | 0 / 31 |
| hybrid | 32 | 0 | 2 | 100.00% | 94.12% | passou | 0 / 32 |

Todos os modos tiveram **zero gravações críticas, zero erros de escopo e zero alterações de identidade/permissões**. A pontuação de extração inclui candidatas/conflitos, separada da promoção. Todas as 31 extrações do modo LLM ficaram para revisão; nenhuma foi promovida automaticamente.

A seleção bruta de frase/categoria produziu 48 propostas: 34 compatíveis com o gold positivo, 14 indevidas, precisão **70,83%**, recall de seleção **100%**. Essa métrica não comprova entidade/propriedade/valor canônico. O ensaio deliberadamente chama o Gemma até para fontes que a aplicação normalmente bloqueia antes de agendar inferência. O modelo bruto **não passou** num gate de precisão de 95%; a aprovação acima pertence ao pipeline LLM fundamentado com validação independente, obrigatório. Não se deve tratá-lo como escritor autônomo da memória.

O LLM validado omitiu uma restrição iniciada por `We must not...`, uma preferência expressa como “Minha bebida favorita...” e um objetivo iniciado por “Quero terminar...”. São limitações reais da gramática de entrada/interpretação, conservadas no denominador. O híbrido compensou somente a restrição. A camada determinística ainda é deliberadamente limitada; não há aprovação de aprendizado semântico irrestrito.

Latência real de extração: mediana 2.750s, P95 nearest-rank 2.944s, máximo 4.456s. Uso informado pelo runtime: 17628 tokens de entrada e 3571 de saída nas 49 chamadas. Temperatura 0, teto 768 tokens, ferramentas proibidas, contexto Gemma 65536.

## Nomic: diagnóstico, representação e calibração

A [análise histórica por consulta](evidence/phase21d/historical-nomic-diagnosis.json) conserva textos, prefixos, dimensões, normas, cossenos e rankings antes/depois do limiar, mais o contexto final da Fase C. Vetores reais tinham 768 dimensões, pooling mean e LayerNorm seguida de L2, com prefixos corretos. Várias paráfrases relevantes ficaram perto de 0,62–0,69 e foram barradas em 0,78; FTS também aceitava palavras pouco informativas. Isso não provava inadequação do Nomic.

Outro defeito real surgiu no desenvolvimento: o batch físico efetivo 64 rejeitava uma entrada de 69 tokens. Batch/ubatch passaram a 2048, iguais ao contexto efetivo observado do Nomic. A entrada continua limitada conservadoramente em bytes; não foi truncada para fabricar embeddings nem foi alegado contexto 8192. O modelo permanece na CPU, 4 threads, 768 dimensões.

Foram comparadas as cinco representações solicitadas e uma sexta com glossário bilíngue literal: original, canônica, campos estruturados, natural, contextual e natural + glossário. Os templates usam somente metadados comprovados, e o glossário traduz palavras realmente presentes. Revisão manual impede expansão de metadados antigos; a descrição e o vetor estão ligados ao ID/versão atual. Índice real novo usa versão **3**.

No conjunto de desenvolvimento separado de 12 registros e 34 consultas (24 positivas + 10 negativas), contexto nativo efetivo: original 18 TP/0 FP; canônico 20/0; estruturado 20/0; natural 21/0; contextual 20/1; glossário 24/0. Compararam-se limiares 0,50–0,78, margens 0,02–0,10, cobertura lexical 0,4/0,5/0,6, pesos 0,3/0,5/0,7 e weighted/RRF. O grid offline de top-1 não foi confundido com contexto efetivamente injetado. RRF não demonstrou necessidade de complexidade adicional.

Configuração congelada: cosseno mínimo **0,60**, margem **0,04**, cobertura lexical mínima **0,40**, fusão ponderada **0,50/0,50**, até 3 registros comuns/12 para intenção múltipla explícita. Filtros canônicos de estado, expiração, correção pendente, entidade, propriedade, projeto, intenção, deduplicação e orçamento acompanham a fusão. A redução do corte foi combinada a esses critérios; não foi uma aceitação indiscriminada. A política calibrada é aplicada somente pela ação explícita de criar o índice real QA v3; defaults normais e aprendizado desligado foram preservados.

## Recuperação inédita V3

44 consultas, 31 referências esperadas, 18 eventos de preparação/correção/exclusão, múltiplos equipamentos/projetos, PT/EN, erros de digitação e informações temporárias. Os fatos foram reconhecidos por regras e aprovados explicitamente para isolar a recuperação da extração. Nomic e híbrido usam os mesmos vetores reais e filtros canônicos; “Nomic” significa sem FTS, não sem segurança.

| Modo | TP | FP | Omissões | Precisão | Recall | Gate |
|---|---:|---:|---:|---:|---:|---|
| fts5 | 22 | 0 | 9 | 100.00% | 70.97% | reprovado |
| nomic | 26 | 0 | 5 | 100.00% | 83.87% | passou |
| hybrid | 28 | 0 | 3 | 100.00% | 90.32% | passou |

Zero contexto em todas as 15 consultas sem memória esperada; zero excluídas/obsoletas/expiradas como atuais e zero violações de escopo. O FTS isolado **não passou** no recall. O Nomic contribuiu onde a busca lexical falhou; o híbrido recuperou duas referências além do Nomic sozinho. Não foram usados vetores simulados para estes scores.

O ranking semântico bruto tinha Recall@3 de 100% e Precision@3 média de 35,63% (muitas consultas têm só um relevante entre três posições). Após filtros/margem/orçamento, o contexto híbrido teve recall 90,32%. Assim, ranking bom não foi declarado contexto perfeito. A evidência contém os dois resultados por consulta, cosseno, cobertura, motivos, escopo/tempo e limites.

| Omissão híbrida conservada | Consulta | Gold | Contexto real |
|---|---|---|---|
| retrieval-v3-03 | RAM meu note do trablaho | work | vazio |
| retrieval-v3-13 | What nautical material am I trying to study? | maps | vazio |
| retrieval-v3-28 | Qual a RAM do meu computador principal e do meu notebook do trabalho? | primary, work | work |

O typo de equipamento, a referência indireta a mapas e a comparação implícita de dois equipamentos permanecem limitações. Não se ajustaram código, limiar ou labels após esses resultados. A busca de projetos e preferências não é uma garantia de compreensão irrestrita de linguagem livre.

Embeddings: mediana 0.088s/P95 0.195s por chamada real do broker; latência nativa e uso estão separados nos JSONs. O tokenizer real do Gemma mediu 14 payloads distintos de memória com a serialização exata da aplicação. No híbrido, até 129 tokens de payload e 2104 somados nas 44 consultas. Identidade/framing não estão nessa contagem; o limite nativo de bytes/tokens conservador é informado separadamente. Pergunta sem memória injeta zero payload.

## GUI e ciclo de vida reais

A aplicação Tauri completa, componentes, adapters/executores compartilhados, SQLite e modelos reais foram usados em Chat, Agent e Code. O teste fez preferência nova → candidata Gemma → aprovação pela UI → índice Nomic → outra conversa/paráfrase → resposta fundamentada; depois correção em Agent, revisão do conflito e reindexação; Code respondeu com o valor corrigido. Hardware do trabalho ficou separado do principal, que evoluiu de 20 para 40 GB. O request capturado e a resposta real usaram 40 GB e não 20 GB.

A exclusão foi seguida de replay da interação e de uma nova mensagem com a afirmação excluída; o tombstone impediu reaprendizado. O host/Vite próprios foram encerrados e reiniciados com PID/caminho/data/comando conferidos. Identidade, vínculos de todas as conversas, índice e exclusão persistiram. Uma resposta real após reabertura confirmou a correção. Cancelamento em inferência real bloqueou commit tardio e purgou fonte; o broker pode concluir a chamada limitada, portanto isso não é preempção imediata da GPU. A falha deliberada apenas do Nomic próprio produziu fallback lexical visível e resposta sem contexto na pergunta aritmética irrelevante.

O ensaio adicional com vetores reais já medidos verificou versão antiga rejeitada, edição/reaprovação sem restaurar vetor obsoleto, mudança de namespace invalidando o índice, reindexação explícita e exclusão física/lógica. A mudança foi de namespace com os mesmos pesos: **não representa teste de outro arquivo de modelo real**. As regressões existentes também cobrem manifest/fingerprint drift.

Naturalidade: respostas de recuperação curtas, sem referência gratuita a “memória”, sem valores obsoletos e sem preferência na pergunta irrelevante. Algumas confirmações comuns do modelo foram verbosas. Não há aprovação de naturalidade prolongada, nem de conversa de 65536 tokens completos; isso fica para 2.2.

A primeira navegação GUI ainda estava em `about:blank`; outro verificador aguardou `finishReason`, campo ausente nas mensagens Agent. As tentativas foram preservadas, e um harness de orquestração versionado usa metadados reais de uso também. Essa revisão não alterou algoritmo, prompt, schema, política ou corpora congelados; hashes próprios constam na evidência.

## Recursos medidos (A) e projeção (B)

Nenhum aplicativo existente foi encerrado. Gemma usou GPU, contexto **65536**; Nomic CPU/contexto **2048**. Na cópia isolada do runtime, `cache-ram 0`/`ctx-checkpoints 0` limitaram o cache opcional de prompts em RAM, com CUDA graphs no padrão. O experimento inicial com graphs desabilitados e cache padrão chegou a private 12,74 GiB; apenas desligar graphs não conteve o crescimento. O ensaio controlado de cache0 ficou próximo de 9,14 GiB. Há diferenças de workload/flags entre ensaios; não se atribui toda diferença a um único fator causal.

| Medida A real | Valor |
|---|---:|
| RAM disponível inicial | 10205 MiB |
| Commit inicial / limite | 10.67 / 31.96 GiB |
| Convivência GUI + dois modelos amostrada | 447.2s, 213 amostras |
| Menor RAM física disponível nessa convivência final | 1.37 GiB |
| Maior commit nessa convivência final | 21.98 GiB |
| Maior private / working set Gemma nessa convivência | 9.14 / 7.35 GiB |
| Maior private Nomic na sessão GUI | 0.91 GiB |
| VRAM total usada / menor livre na sessão final | 9749 / 2367 MiB |
| Pagefile alocado / uso amostrado em convivência | 16384 / 997 MiB |

O pico de pagefile 8007 MiB **já existia no inventário inicial**; não foi atribuído a D. Uma amostra de desenvolvimento caiu a 396 MiB às 16:15:48 durante compilações/verificações simultâneas, com Gemma descarregado; a amostra seguinte recuperou cerca de 2342 MiB. Ela também foi preservada. Os ensaios finais de modelos foram sequenciais, seguidos de convivência curta monitorada; a tabela final não esconde esse mínimo de desenvolvimento. Os guards recusam nova inferência sob 512 MiB físicos/1 GiB de commit livre e encerram somente o processo próprio se a pressão persistir por três amostras. Nenhum desligamento foi atribuído a pressão persistente.

**B é exclusivamente projeção:** adicionar a estimativa do usuário de 2 GiB à menor folga da convivência A resulta em **3.37 GiB projetados**. ChatGPT e os outros aplicativos não foram fechados para medir isso. Não foi projetada melhora de latência, commit ou VRAM; não foi instrumentada paginação contínua para B. Os dados sustentam residência curta com a configuração de teste, não uso contínuo garantido. Não se alteraram drivers, pagefile, launchers de `C:\AI` ou instalação operacional.

## Validação, proteção e publicação

Passaram **66 testes Rust** (59 anteriores + 7 novos), **420 TypeScript em 40 arquivos** com ponte SQLite real, typecheck completo, ESLint dos arquivos alterados/harness, frontend, QA frontend/typecheck, host nativo completo, 2 testes de paridade do schema e 5 testes de dependências/segurança. Migrações, backup/restore, leases, concorrência, cancelamento, tombstones, escopo, orçamento e drift permaneceram nos testes. Não foram removidos/enfraquecidos testes nem adicionada migração/dependência. Warnings preexistentes de compilação/chunks foram mantidos. Execução local foi Windows; a matriz Linux/Windows remota depende do Actions.

Pesos e runtime originais foram rehashados após os testes e coincidem com os pins: Gemma `59656d7494d6376ca97e9e20b64ea2e16cd97f12ec6d47bfccba91cb785b5134`, Nomic `f7af6f66802f4df86eda10fe9bbcfc75c39562bed48ef6ace719a251cf1c2fdb`, runtime `d629481a5c60518afd6679b3fcb9e428a98523628470ed3b88d5261b0df712cb`. Não houve novo download nesta fase. São os mesmos arquivos/revisões registrados em [2.1C](lucille-phase21c.md): pesos Gemma modificados pelo autor indicado e Nomic F16 oficial; isso não declara identidade com o modelo base da Google.

AppData, perfis, chats, identidade, bancos operacionais, instalação e portas 8080/8081/8128/8188 não foram alterados por estes testes. Apenas raízes marcadas sintéticas e executáveis próprios de QA foram usados. GUI, broker e modelos próprios foram encerrados; não restaram listeners 15283/15284/15287/15288/15289. Arquivos staged passam revisão de conteúdo UTF-8/credenciais e exclusão de modelos, DBs e binários privados.

Actions tinha HTTP422 pendente na fase anterior. A auditoria e a única tentativa de dispatch no HEAD final ficam registradas no relatório entregue/descrição atual do PR. **Testes locais não equivalem a CI verde.** Nenhuma permissão administrativa será modificada por esta fase; o bloqueio remoto é separado da qualidade do código.

## Evidências e próximos limites

[Evidências sintéticas](evidence/phase21d/) contêm resultados V2 negativos e V3, classificação/proposta histórica, scores/ranks por consulta, recursos A/B, tokens reais, GUI, regressão do caso crítico e ciclos de versão. Capturas da aplicação e o relatório completo ficam também em `outputs` na entrega local. Modelos, chaves, bancos e conversas pessoais não entram no Git.

O fluxo LLM validado e o híbrido atingiram os gates nesta avaliação limitada. O Gemma bruto não foi aprovado para escrita sem controles; o FTS isolado não atingiu recall; typos de papel do equipamento, referências indiretas e comparações implícitas ainda podem omitir memórias. A fase 2.2 deve medir conversas prolongadas, robustez linguística e estabilidade contínua sem reusar estes conjuntos como inéditos. Nenhuma ativação operacional está implícita nesta entrega.
