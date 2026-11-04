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

## Confirmação durável e consulta de ausência — desenvolvimento

A leitura nativa de answer_evidence agora pode devolver um recibo de aprendizagem somente quando o job da fonte original está done e há registros candidate/active vinculados à sua proveniência, no mesmo escopo/projeto. A transação de leitura observa apenas commits; reserva, running, cancelamento e uma memória antiga de outra fonte não bastam. O recibo distingue estados por registro e não concede prova de atualização/reativação. O verificador mantém a confirmação de estado canônico já existente separada da alegação de uma nova operação. Não foi criada tabela ou migração.

A fronteira comum de resposta reconhece confirmações elípticas como Registrado e operações de atualização. Candidata persistida não autoriza declarar memória ativa. Leitura renovada imediatamente antes da exposição e buffering continuam comuns aos três modos; a GUI real ainda será medida.

Afirmações categóricas de não recebimento agora acionam confirmação canônica por entidade/propriedade, incluindo a formulação forneceu/provided. Uma consulta sem valor representa leitura do domínio tipado, não um hit de recuperação e nem prova de que a informação nunca foi fornecida. Respostas sobre GPU continuam independentes de registros de RAM. O aumento de RAM liga a quantidade ao predicado de diferença e a um único equipamento com predecessor literal; não interpreta o delta como capacidade total.

O [replay de respostas antigas](evidence/phase22e/development/verifier-operation-replay.json) rejeita a confirmação prematura e a falsa ausência e aceita o delta correto, sem nova inferência. Snapshots diagnósticos projetados permanecem explicitamente identificados. Os 142 testes Rust passaram, incluindo os 135 anteriores inalterados. Os cinco arquivos TypeScript direcionados passaram com 112 testes. Verificação TypeScript completa e typecheck em execução, sem alegação de CI remoto.

Dois problemas encontrados nos próprios testes de desenvolvimento foram corrigidos antes de prosseguir: a política legada também promovia preferências, portanto o fixture de candidata agora desabilita auto_kinds explicitamente; a primeira alteração da consulta SQL não alcançou o predicado sem alias, e o novo teste nativo de lookup detectou a falha. Logs de tentativas preservados localmente. Nenhum conjunto final foi criado ou congelado nesta etapa.

## Recuperação composta — desenvolvimento

A consulta composta é dividida em relações com ator e intenção temporal próprios. Cada relação passa pelos filtros e limites existentes, no mesmo escopo/projeto autorizado; resultados são reunidos sem duplicatas, com teto de doze referências e orçamento total. Domínios elípticos podem herdar o nome da propriedade, nunca a autoria, o projeto nomeado ou a modalidade de outra relação. Nenhum limiar global foi reduzido. A similaridade Nomic conserva o embedding real da pergunta inteira como prior global de ranking, com filtros locais por relação; não é apresentada como um novo embedding de cada fragmento. Com Nomic indisponível, cada relação usa o fallback lexical nativo.

O contexto inclui, por relação, referências e estado retrieved/unavailable, que significa somente disponibilidade neste contexto. A instrução comum de Chat/Agent/Code pede resposta independente a cada relação. O verificador rejeita transferência de fatos pessoais ao irmão e detecta omissão da relação própria conhecida. O outro projeto continua fora do projeto autorizado: sua indisponibilidade não apaga o banco do projeto corrente.

Regressões dirigidas recuperam o banco próprio em comparações com irmão em ambas as ordens; mantêm duas capacidades de RAM; separam escolha de Rust e implementação em Python; preservam assunto do catálogo, banco e linguagem em três relações. A suíte anterior identificou duas perdas de contexto da decomposição: o domínio projeto na pergunta histórica e a elipse principal na comparação de equipamentos. Foram corrigidas mantendo os testes antigos intactos. Os 143 testes Rust passaram na repetição completa; a suíte TypeScript dirigida passou com 113 testes. A validação real final continua pendente.

## Paráfrases e esquecimento real — desenvolvimento

A preferência nominal em inglês agora interpreta o complemento de preparação com um adjunto limitado de consumo em primeira pessoa. A seleção de classes reconhece select como predicado do usuário, com autoria comprovada na fonte. Na oração final anteposta sobre PostgreSQL, o problema restante era a classificação de decidi após projeto, como suposto nome de outro projeto; a fronteira de cláusula agora permite o início verbal pessoal, preservando nomes explícitos e citações.

O [novo replay nativo dos cinco outputs originais](evidence/phase22e/development/paraphrases-replay.json) cria exatamente um registro canônico para dx-002, dx-010 e dx-016 em cada modo. dx-031/dx-032 continuam com zero candidatas. Mesmos textos, escopos, projetos e respostas reais antigas; sem nova inferência ou holdout.

O imperativo de esquecimento admite a descrição relativa própria jogos em que escolho a classe. O teste nativo comprova exclusão efetiva, tombstone persistido, recibo de forget com had_records=true, replay de conclusão antiga sem escrita, ausência após reinício, nova fonte explícita, candidata com nova proveniência, revisão e persistência ativa após outro reinício. O primeiro teste esperava incorretamente erro para uma conclusão já encerrada: o contrato real devolve false idempotente, agora validado explicitamente junto de ausência de ressurreição. Não foi confundido com voltar a gostar.

Os 145 testes Rust passaram, incluindo todos os 135 anteriores; bridges recompilados. Novas inferências e GUI real da fase continuam pendentes.

## Fidelidade e integração da preferência simples — desenvolvimento

A decisão de trocar uma tecnologia agora pode comprovar os extremos da intenção a partir da própria declaração USER nativa, sem exigir que o extremo de origem seja uma implementação anterior recuperada. A troca efetivamente realizada, uma origem invertida ou um estado anterior inventado continuam sem suporte. A negativa correta não uma capacidade total de 128 GB preserva sua negação; uma capacidade positiva de 128 GB e uma negativa ambígua entre dois equipamentos não são autorizadas. Consequências qualitativas sobre margem de multitarefa e desenvolvimento fluido exigem evidência além de quantidade de RAM; hipóteses explícitas e negativas de garantia permanecem separadas.

A preferência simples por café agora participa também de recuperação, resposta e esquecimento. Não prova açúcar nem bebida favorita. Esquecer o tópico café remove de fato a preferência simples e preserva tombstone; não deixa esse novo campo ativo por esquecimento restrito ao antigo coffee_sugar.

Uma leitura canônica adicional encontrada durante a verificação é fornecida como dado não confiável à única regeneração real. Não basta consultar o banco e depois pedir ao modelo que responda novamente com o mesmo contexto inicial vazio. Cada tentativa guarda a evidência nativa de operação realmente usada antes da exposição, inclusive recibos ausentes, para auditoria temporal na GUI.

Validação local desta etapa: 146 Rust (135 anteriores intactos), 721 TypeScript (711 anteriores intactos), três testes do esquema atômico. Os arquivos TypeScript dirigidos passaram com 117 testes. Testes do próprio avaliador confirmam contrato active mais validade histórica, rejeição explícita da antiga suposição obsolete e denominadores que conservam casos ausentes/bloqueados. Nenhuma destas aprovações técnicas substitui a avaliação real final.

## Avaliador e host isolado 2.2E

O [preflight nativo dos 21 casos conhecidos](evidence/phase22e/development/evaluator-preflight.json) reproduziu precisamente a falha da-017: Rust de angico permanecia active/in_progress, qualificado como histórico pelo cancelamento. Vinte seletores estavam prontos; um exigia obsolete incorretamente. Somente a cópia de desenvolvimento da 2.2E foi corrigida para active com projeção historical/in_progress. O segundo preflight alcançou 21/21 pontos de execução, sem inferência. A 2.2D permanece com seus resultados originais, inclusive os cinco casos não executados.

O novo avaliador retém cada caso previsto mesmo se uma pré-condição falhar; disponibilidade e completude conservam os denominadores planejados. Casos mistos permitem contabilizar os falsos positivos críticos sem classificar a declaração própria legítima como violação. A auditoria literal adicional compara proveniência e intervalo original com spans USER autorizados no gold independente; um valor canônico correto não mascara proveniência de personagem ou terceiro. Três testes do contrato e um teste da rubrica literal passaram antes de qualquer novo conjunto final.

O host, os controladores e o observador têm raiz/perfil exclusivos 22e. O observador mede conteúdo real, DOM de Chat/Agent e transcript de Code, recibos nativos e estados dos jobs; reconhece erros de transporte dos três modos sem depender de metadados de término ausentes no Code. A evidência de confirmação usada na verificação é registrada antes da exposição. As execuções continuam usando conclusão não streaming adaptada a SSE no broker de QA; não comprovam streaming nativo do Gemma.

Build normal do frontend e build estático isolado concluídos; host nativo recompilado com modelos desligados. Uma advertência preexistente do plugin nativo e avisos de tamanho/divisão dos bundles permanecem. Inventário privado de 104 arquivos protegidos foi feito por hash em leitura antes dos testes GUI/modelos desta fase. Nenhuma instalação operacional foi aberta pelo Store nativo, migrada ou atualizada.

## Validação real de desenvolvimento e segunda correção de autoria

A primeira execução real E repetiu os 40 casos conhecidos D com Gemma: regras e híbrido 25 TP, zero FP, zero perdas; LLM 24 TP, zero FP, uma perda (dx-034). Zero candidatas críticas, promoções erradas ou violações de escopo observadas. Este corpus antigo não contém os spans independentes exigidos pela nova rubrica; estes números são regressões conhecidas, não avaliação inédita de origem.

A primeira conversa GUI E previa 36 turnos. Sete foram registrados; a oitava tentativa Agent foi confundida pelo observador com falha de transporte, embora fosse bloqueio de fidelidade. Nenhum dos 13 probes foi executado. As saídas incorretas que atribuíram chá ao USER foram retidas. Duas respostas legítimas também foram retidas: relato da narradora e contraste USER café/personagem chá. O verificador agora separa sujeitos contrastantes e não vincula relato explicitamente externo ao USER; a referência nominal sua preferência por chá continua sendo verificada como pessoal. O observador conserva bloqueios como resultados de produto, sem interromper como erro de rede.

Um diagnóstico derivado do cabeçalho conhecido, removendo os dois-pontos, encontrou outra falha nativa: a linha seguinte retomava autoria USER e criou uma preferência ativa. O parser passou a conservar a autoria de cabeçalhos externos sem pontuação. Comentários comuns sobre documentos não ganham esse efeito. A prova nativa tem versão explícita 2; provas pendentes antigas sem a versão atual não concedem promoção, enquanto memórias canônicas existentes continuam legíveis. Não houve migração SQLite. Conclusões híbridas voltam a validar também as propostas por regras contra a fonte integral disponível.

No diagnóstico controlado após a correção, três modos rejeitaram a preferência do documento e preservaram somente a preferência pessoal independente antes dele. As propostas estruturadas desse diagnóstico foram construídas explicitamente: não são novas inferências. Uma tentativa intermediária tinha escopo de proposta incompatível e foi rejeitada antes da extração; não foi contada como sucesso de autoria. Fontes, resultados e falhas anteriores estão preservados na [evidência de desenvolvimento](evidence/phase22e/development/development-real-v1-and-header-v2.json).

A contagem nativa de falantes acompanha agora a preparação do contexto e orienta a geração/regeneração, sem transferir ao modelo permissão para gravar. O custo dessa orientação é descontado do orçamento antes da recuperação. Um bloqueio histórico distinto também foi corrigido: predicados descritivos sobre recursos de jogos não herdam a preferência do USER apenas porque a pergunta era pessoal. Declarações pessoais explícitas e respostas elípticas continuam verificadas.

Validação técnica atual: 147 Rust (135 anteriores), 723 TypeScript (711 anteriores), três testes de esquema, três de contrato do avaliador e um da rubrica de origem. Os resultados reais parciais acima não aprovam a fase. A GUI está sendo recompilada com os modelos desligados para nova execução de desenvolvimento em escopo novo, sem sobrescrever a execução interrompida.
