# Lucille 2.2E — diagnóstico e contrato de origem

Base local/remota `92f75a0d661fa9804b468e55b3760090b47bdd66`, conferida antes de alterações. PR12 aberto, draft, HEAD correspondente. Working tree rastreada limpa; somente cache Python não rastreado, não publicado. Branch `feature/lucille-phase22e-source-authority`, empilhada sobre 2.2D. Nenhum modelo foi carregado para estes replays; nenhum banco operacional foi aberto.

## Diagnóstico anterior à mudança de comportamento

Os executáveis 2.2D foram copiados antes de novos builds. SHA e dados de fronteira estão em [replays originais](evidence/phase22e/baseline/original-replays.json). O exemplo `authority_diagnostic` só chama funções puras existentes, não abre Store, não chama modelo nem altera políticas. Os testes de fronteira usam propostas explicitamente controladas, **não respostas Gemma simuladas ou novas inferências**. Os replays dx usam as respostas reais anteriores, inalteradas.

| Caso | Evidência e validação | Política/persistência | Causa demonstrada |
|---|---|---|---|
| dx-031, ferramenta informa preferência em primeira pessoa | Span original completo; validação aceita subject=user, favorite_beverage=tea | Active em rules/llm/hybrid; proveniência conserva literalmente a atribuição à ferramenta | Mesmo com o qualificador presente, mensagem USER é confundida com declaração pessoal. Não é apenas truncamento de trecho |
| dx-032, narradora de diálogo inventado | Span original completo; validação aceita subject=user, coffee_sugar=without_sugar | Active nos três modos | Predicados de primeira pessoa internos são classificados como pessoais; autoria externa não acompanha Claim |
| Cabeçalho de narradora/documento + próxima linha em primeira pessoa | Mensagem inteira passa eligible_source; extract/normalize reinterpreta a linha isolada | Preferência Active nos três modos | Perda adicional de contexto entre o cabeçalho e o span. normalize chama extract(span); promoção revalida apenas Claim.evidence |
| Declaração própria junto de irmã/citação/log | Filtro global rejeita mensagem ou predicado misto não é decomposto | Zero registros, inclusive o café legítimo | Gate de mensagem inteira e classificador de cláusula têm granularidade incompatível com autoria por afirmação |
| dx-002 / dx-010 / dx-016 | Gemma original nomeia evidência; nominal inglês e oração anteposta falham no gate ou na interpretação | Zero registros nos três modos | Falhas permanecem distintas de erro do modelo |
| Registrado no turno histórico 92 | Texto real anterior, snapshot diagnóstico com declaração USER atual e sem memória persistida: zero violações | Prova original de pintura/commit permanece em 2.2D | Confirmação elíptica de operação não reconhecida; declaração atual não é recibo |
| Ainda não forneceu detalhes do banco planejado | Texto real anterior, snapshot com plano SQLite conhecido: zero violações | Plano nativo existe; atribuição de ausência passa | Verificação categórica de ausência não reconhece esta relação e não consulta o domínio canônico correspondente |
| Delta 32 → 56 GB | Texto real correto: aumentou em 24 GB; duas máquinas conhecidas e correção atual identificada | unsupported_typed_value crítico | Quantidade derivada é confundida com capacidade, sem preservar relação de mudança |

[Fronteiras instrumentadas](evidence/phase22e/baseline/boundary-diagnostics.json), [verificador sem alteração](evidence/phase22e/baseline/verifier-replay.json). Os snapshots de resposta são projeções explícitas das relações originais, não uma alegação de captura do payload integral antigo. A primeira tentativa de replay mudou scope, causando rejeição correta do JSON antigo: artefato inválido preservado, nenhuma conclusão de segurança extraída; replay válido usa scope/project originais em outros bancos novos.

## Três autoridades distintas

O papel de transporte já é validado como USER na reserva/publicação nativa. Isso não identifica o falante de cada afirmação. Hoje o classificador produz subject=user e a representação Evidence inicializa actor=user/origin=user_clause; a promoção toma concordância de interpretação por prova pessoal. A proveniência registra a fonte literal, mas não corrige esse erro anterior.

A correção deverá resolver **na fonte completa** os intervalos das afirmações: autor da mensagem, falante, sujeito e origem. A proposta LLM continua restrita a evidência/categoria e não escolhe esses metadados. Fronteiras de citação, discurso relatado, cabeçalhos e coordenação devem preservar intervalos UTF-8 originais e permitir uma declaração própria ao lado de conteúdo externo. A presença isolada das palavras personagem/log/ferramenta não será motivo para rejeitar o restante da mensagem.

Preferir acrescentar prova de origem por Claim à representação intermediária e à validação nativa, com vínculo ao digest da fonte original, intervalo literal e campos canônicos. Construção e revalidação serão nativas; propostas não podem aumentar sua própria autoridade. Claims positivamente atribuídos a outro falante não podem virar candidatas pessoais. A promoção também deve exigir autoria comprovada, e não só uma expressão em primeira pessoa em Claim.evidence. Verificar regras/LLM/híbrido e preservar os fences de fonte/lease e ordem de correções.

Não há necessidade demonstrada de nova tabela ou schema. Metadados de auditoria podem usar as estruturas JSON já existentes e a regra de evidência versionada. Compatibilidade de jobs antigos sem contexto/prova exige tratamento explícito conservador, sem presumir que texto isolado comprova autoria. Nada será instalado no perfil operacional.

## Ordem e validação

1. Corrigir origem e os dois caminhos demonstrados; testar negativos e a parte própria de mensagens mistas em português/inglês, incluindo o café sem qualificação de açúcar sem inventar preferência por açúcar ou bebida favorita.
2. Confirmar fronteiras nativas de promoção, candidata, cancelamento, retry e replay antes de tratar confirmação/ausência/recuperação.
3. Corrigir relação de operação concluída versus fonte recebida, ausência por domínio, mudança de valor e recuperação de relações compostas. Validar tombstone real e reintrodução após reinício.
4. Auditar o avaliador e a representação de erro de Chat/Agent/Code antes do novo congelamento, inclusive o antigo pressuposto obsolete versus active+cancelled. Verificar todos os seletores e denominadores sem ensaiar respostas do futuro holdout.
5. Preservar os 135 Rust, 711 TypeScript e três testes de esquema. Builds pesados com modelos desligados; Gemma 65.536, Nomic CPU/768/2.048, cargas serializadas e processos próprios identificados. Sem CUA da tela do usuário.
6. Somente após encerrar produção, criar/versionar novo conjunto e congelar código, avaliadores, parâmetros, pesos/runtime, identidade/políticas, executáveis e frontend. Não ajustar após os resultados. Comparar com conjuntos antigos imutáveis e séries 20 → 50 → 100 no mesmo escopo original. Aprovação depende de todas as metas e zero críticas; testes técnicos sozinhos não aprovam.

CI permanece separado: consultas de leitura, sem dispatch repetido, alteração administrativa, merge ou afirmação de verde sem runs. Evidências públicas somente sintéticas; fontes, tokens, leases e logs privados permanecem locais.

## Primeira correção de origem — desenvolvimento

A representação intermediária Claim agora recebe SourceAuthority calculada exclusivamente a partir da mensagem original nativa: autor de transporte, falante, sujeito, classe de origem, digest da fonte, intervalo UTF-8 e vínculo aos campos canônicos. A seleção de evidência pelo modelo só restringe afirmações já atribuídas nativamente; não reinterpreta o trecho isolado como uma nova declaração USER. Cada afirmação é revalidada antes de candidatura, promoção e conclusão, com vínculo ao job reservado e aos fences existentes.

A auditoria compacta usa Quality.evidence_rule e o JSON de filas existentes, mantendo schema 5. Jobs pendentes antigos sem prova original falham de modo conservador; não recebem autoria presumida durante restore. Memórias canônicas existentes permanecem legíveis. A prova é um contrato nativo de origem, não uma assinatura contra um administrador capaz de forjar integralmente o banco.

O replay de desenvolvimento em [source-authority.json](evidence/phase22e/development/source-authority.json) conserva as mesmas cinco respostas reais anteriores do Gemma e identifica separadamente nove propostas controladas. dx-031 e dx-032 passaram de uma memória ativa por modo a zero candidatas em rules/llm/hybrid. Os cinco positivos mistos preservaram exatamente uma preferência própria em cada modo; café simples é beverage_preference=coffee, sem inferir açúcar ou bebida favorita. As três paráfrases antigas continuam sem extração nesta etapa. Não houve nova inferência nem aprovação de holdout.

Validação local: 140 testes Rust aprovados, incluindo os 135 anteriores inalterados e cinco testes novos. Incluem autoria mista/externa, intervalo UTF-8, tentativa de autoridade fornecida pelo modelo, corrupção de vínculo ao job e falta de prova em fila antiga. Bridges e diagnóstico recompilados com modelos desligados. GUI, TypeScript completo e validação real final da 2.2E ainda pendentes.
