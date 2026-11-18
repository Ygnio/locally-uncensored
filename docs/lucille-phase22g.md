# Lucille 2.2G — checkpoint diagnóstico

**Decisão: CONTINUAR, com um próximo ciclo dirigido.** O fallback nativo passou de 17/21 para 20/21 casos obrigatórios. As três fontes que bloquearam 11 consultas C2 passaram a materializar fatos tipados. O Gemma resolveu 13 das 15 perguntas com contexto ideal, mas uma reformulação transformou escolha em uso efetivo. O verificador aceitou os dois erros desse conjunto. O investimento indicado agora é testar materialização e discriminação de relações em casos novos. A reprovação operacional da 2.2F continua definitiva.

## Quanto cabe ao software e ao modelo

| Ocorrência na 2.2F | Último sucesso e primeira falha | Atribuição e confiança |
| --- | --- | --- |
| Três preparações C2 | O avaliador enviou a declaração íntegra ao Gemma e recebeu proposta literal válida. O código rejeitou a fonte em `enqueue` antes do job. Os 11 consumidores ficaram sem execução. | **A, alta**. Onze perdas de disponibilidade entre 82 casos têm essa causa comum. São efeitos de três rejeições. |
| Quatro erros obrigatórios do C1 offline | Registros sintéticos tipados e revisados, seleção exata incorreta, com e sem o serviço de embeddings. | **C, alta**. Nenhuma geração de resposta participa dessa métrica. |
| Transporte Chat e Code | Ambas as fontes USER sobreviveram. Chat teve proposta válida, job concluído e zero registros. Code preservou a fonte e não teve job nem chamada de extração. | **B em Chat, A na admissão de Code**. Replay nativo e teste com recibo sustentam a atribuição. O transporte GUI completo ainda precisa de reteste. |
| RAM em `fv1-main-t15` | O broker entregou as duas frases ao extrator em modo LLM. As quatro propostas citaram exclusivamente «Os 20 GB descreviam a configuração anterior». Job concluído sem recibo. As duas respostas conversacionais continham 36 GB e uma declaração de atualização sem respaldo. | **D na nomeação, alta neste caso**. O verificador reteve corretamente as duas respostas. O antigo registro de 20 GB seguiu ativo. Em `fv1-main-t16` o modelo respondeu corretamente 36 e 14 GB, mas o dado persistente continuava desatualizado. |
| Autoria em `fv1-main-t14` | A resposta identificou o café corretamente e atribuiu ao usuário o tema do material fictício. Só a bebida recebeu vínculo de evidência. A resposta inteira foi exibida. | **E comprovada**, com erro na geração bruta. O caso isolado não mede a capacidade geral do modelo com a fonte ideal. |
| Persistência em `fv1-forget-t01` | A resposta reconheceu o tema escuro e prometeu mantê-lo para configurações futuras aos 2.684 ms. O job estava na fila. Primeira leitura do recibo aos 7.624 ms. | **E, alta**. A promessa ultrapassou a evidência disponível no momento da exibição. |
| Respostas retidas | Em `fv1-main-t03`, uma explicação geral sobre fatores de desempenho acionou a regra de desempenho pessoal. Em `fv1-main-t12`, a preferência de Cecile acionou a checagem de valor pessoal. Em `fv1-main-t21`, as duas tentativas afirmaram uso de SQLite após cancelamento. | Retenções indevidas nos dois primeiros exemplos, retenção justificada no terceiro. A perda de utilidade agrega situações diferentes. |

Uma divisão percentual global misturaria consultas não executadas, preparações compartilhadas, respostas e afirmações. A evidência sustenta contagens por etapa e admite causas simultâneas. Em C2, cinco seleções híbridas medidas divergiram do gabarito, incluindo o violão persistido que não chegou à resposta. A matriz por caso traz os textos USER, os resultados esperados e observados e as dependências. O complemento GUI traz tentativas, códigos de verificação e estados de job e recibo observados. [Auditoria](evidence/phase22g/audit.json) · [Extratos GUI](evidence/phase22g/gui-trace.json) · [Propostas originais e hashes das linhas de log](evidence/phase22g/nominations.json)

## Teto prático observado

24 chamadas locais ao mesmo peso e executável da 2.2F, conferidos por SHA-256. O diagnóstico usou contexto de 4.096 tokens, temperatura 0,2, seed 42, máximo de 160 tokens de saída, raciocínio desativado, uma requisição por vez. As entradas couberam integralmente na janela e todas terminaram em `stop`. A 2.2F operacional usava contexto 65.536 e instruções e formatos próprios. O oracle usou fatos sintéticos no prompt, sem SQLite, FTS5 ou Nomic. A adjudicação abaixo é uma leitura semântica das saídas arquivadas pelo autor deste checkpoint.

| Capacidade | Evidência observada | Confiança e limite |
| --- | --- | --- |
| Sujeito, autoria fictícia, negação e hipótese | Os seis exemplos de extração preservaram o sentido principal. Três repetições mantiveram a relação. Em x5 e sua repetição, «cheia/carregada» aparece como propriedade e «adoro/detesto» como valor. | Moderada para compreensão nesses exemplos. Com esquema estrito, x5 é parcial nas duas tentativas. O teste em linguagem livre não mede validade do JSON operacional. |
| Dispositivos e temporalidade | x3 e sua repetição distinguiram portátil, valor antigo e novo do desktop. As perguntas básica, explícita e reformulada sobre RAM atual acertaram. | Moderada no contexto curto. A resposta explicit2 acrescentou uma falsa ausência de histórico apesar dos 48 GB fornecidos. |
| Escolha, intenção, implementação e uso | x1 e x2 distinguiram os estados. basic3 e explicit3 mantiveram uso desconhecido. variant3 declarou que a escolha provava uso. | Falha de geração demonstrada sob reformulação. Baixa confiança para declarar limitação persistente. |
| Ausência de dados e isolamento | basic4, explicit4 e variant4 reconheceram falta de dados autorizados sobre Sal. | Moderada no cenário testado. O modelo recebeu somente fatos de Lume, condição diferente de testar um filtro real entre bancos. |
| Resposta com contexto ideal | Prompt curto **6/6**. Instrução explícita, mesmos fatos e perguntas, **5/6**. Reformulações **2/3**. Total **13/15** respostas integralmente sustentadas. | Amostra pareada pequena. Uma instrução extra não demonstrou ganho. Nenhuma estimativa populacional ou teto absoluto. |
| Verificador real | O caminho `validateLucilleCompletion` recebeu as 15 saídas e os mesmos fatos em snapshot tipado. Entregou as 15 literalmente na primeira tentativa, incluindo a falsa ausência e a inferência de uso. | **13/13 corretas preservadas, 0/2 incorretas contidas**. Sem omissões pedidas, trocas de autoria, retenções ou confirmações de persistência nessas 15 perguntas. A GUI congelada testa operações e autoria com casos próprios. |

O melhor resultado observado foi 6/6 nas perguntas originais. Ele coexistiu com falhas sob mudanças pequenas de instrução e formulação. Não há base para trocar o modelo principal. O gargalo reproduzido na extração de RAM operacional merece uma comparação de política: em banco vazio, a mesma proposta arquivada criou zero fatos em modo LLM e uma candidata correta de 36 GB em híbrido. Ainda falta testar o conflito, a aprovação e a substituição com a versão antiga já ativa antes de qualquer mudança de política. [Entradas, parâmetros, gabaritos, respostas e uso](evidence/phase22g/benchmark.jsonl) · [Replay do verificador](evidence/phase22g/verifier.json)

## Alterações e efeito medido

- **Admissão e autoria.** O reconhecimento nativo aceita verbos de primeira pessoa já compreendidos pelo extrator. A função diferencia o rótulo «Sistema:» do substantivo em «no sistema de reservas». As três propostas C2 originais, reproduzidas em híbrido, terminaram com job concluído e o fato esperado. Crivo e Nacar ficaram como candidatas e a preferência negativa ficou ativa. Testes adicionais trocaram projeto e banco e incluíram fontes fictícias e condicionais.
- **Transporte.** A normalização nominal recupera «explicações detalhadas». O classificador de autoria preserva o uso de «play role-playing games» como atividade do usuário. O teste de publicação nativa vinculou os recibos à fonte nos dois modos. O replay da proposta real de Chat, em modo LLM, produziu preferência ativa. O indicador GUI formal de transporte continua em **1/3** até novo teste com falha injetada.
- **Recuperação.** O tratamento de RAM separa uma pergunta só pelo valor anterior de uma comparação entre passado e presente. A pergunta elíptica herda o dispositivo quando o vínculo é unívoco. A seleção de preferências distingue idioma de estilo e reconhece «escolho». O avaliador original de C1, em bancos sintéticos novos e sem vetor, executou **26/26** consultas. Acertou **25/26**, incluindo **20/21** obrigatórias. Corrigiu f1q-06, f1q-19 e f1q-22 e preservou todas as seleções anteriormente exatas. Em f1q-16 continuou faltando a implementação. Duas tentativas sobre a divisão da pergunta não corrigiram o resultado e foram retiradas. O próximo experimento deve comparar as referências de cada subconsulta antes e depois dos filtros de modalidade.
- **Verificação.** O teste do texto de `fv1-main-t14` passou de aceitação a violação de autoria. A promessa em `fv1-forget-t01` e uma variante agora acionam a regra de promessa sem confirmação. O controle limitado à resposta corrente continua aceito. Esses testes medem a decisão do verificador. A disponibilidade do texto útil após regeneração na GUI ainda não foi medida.

Os testes direcionados passaram em **24/24 Rust** e **92/92 TypeScript**. O replay nativo falha se perder um caso anteriormente exato de C1 ou qualquer uma das três correções de recuperação. [Resultados nativos](evidence/phase22g/native.json) · [Replay reproduzível](../scripts/replay-lucille-phase22g.py)

## Custo, decisão e trabalho restante

O Gemma registrou **2.285 tokens de entrada, 1.416 de saída e 45,415 s acumulados nas requisições**. A duração exclui inicialização, hashing e testes. O replay do verificador, a auditoria e os replays nativos consumiram zero inferências adicionais. Nenhum modelo foi baixado. Não há medição direta do consumo de tokens do Codex por etapa nem de energia ou pico de VRAM desta rodada. Os picos de hardware registrados na 2.2F pertencem àquela execução.

A referência formal permanece em 90% de recall híbrido de extração, C2 16/27, 11 ausências, GUI+C2 54/82 úteis, 60/73 afirmações sustentadas, sete críticas e uma promessa prematura. Materializar as três fontes não demonstra que as 11 consultas passaram. Os critérios originais de 95% de disponibilidade e fidelidade, 100% de fallback obrigatório e zero críticas continuam em vigor.

**Estimativa para a próxima decisão:** dois ciclos curtos de engenharia e um reteste de integração. Primeiro, rastrear f1q-16 e executar as 11 dependências C2 em perfis isolados, reaproveitando as gerações ou embeddings existentes quando correspondam exatamente à entrada. Segundo, testar autoria e modalidade com contraprovas novas e comparar LLM e híbrido com RAM anterior ativa. Por fim, retestar Chat e Code com falha de transporte e observar DOM, job, registro e recibo. Como ordem de grandeza, reservar 1–2 dias de engenharia e até 12 novas chamadas para esse reteste dirigido, sujeito ao resultado offline. O esforço até aprovação operacional ainda é indeterminado. Uma nova campanha longa depende de evidência de resolução das falhas críticas.

## Consolidação funcional de 18 de novembro de 2026

Checkpoint dirigido sobre `3804049`. A avaliação formal 2.2F continua congelada e reprovada. Nesta rodada, a contagem de recuperação corresponde à seleção de referências canônicas, e a contagem de respostas corresponde a gerações registradas no broker local.

| Componente | Anterior | Atual | Situação |
|---|---|---|---|
| Fallback C1 | 20/21 obrigatórias | 21/21 obrigatórias e 26/26 exatas | Passou nas consultas congeladas |
| Preparação C2 | 3 fontes recuperadas isoladamente | 20/20 fontes USER materializadas, 12 revisões explícitas e 2 mudanças de estado confirmadas | 18 índices ativos, 2 históricos ou excluídos fora do índice atual |
| Recuperação C2 | 11 dependentes pendentes | 27/27 exatas em fallback e híbrido, inclusive as 11 dependentes | 25 referências esperadas em cada modo, zero extras, omissões ou invasões de escopo |
| Respostas C2 | 11 dependentes pendentes | 9 primeiras gerações, 8 entregas, 5 respostas adequadas e 1 bloqueio | Fidelidade integral pendente |
| Transporte GUI | 1/3 | 3/3 na aplicação Tauri de QA | Chat, Agent e Code passaram com falha injetada e recibo vinculado ao registro |
| Atualização de RAM | Substituição falhava com 20 GB já ativos | Rules, LLM e Hybrid passaram com 20 GB históricos e 36 GB atuais após revisão | Passou no caso dirigido com reabertura do banco |
| Fidelidade com contexto ideal | 13/15 brutas e 2 erros aceitos | Mesmas 13 corretas preservadas e os 2 erros bloqueados | Zero retenções úteis nesses 15 replays |

### Fontes, revisão e recuperação

Em `f1q-16`, a busca de RAM por substring encontrava «ram» dentro do nome de um projeto. Isso tornava ambígua a propriedade que a segunda oração deveria herdar. A decomposição agora reconhece imperativos e perguntas preposicionais, preserva as modalidades de escolha e execução e exige RAM como token. O rastreio compara o binário congelado da F e o bridge da G. [Seleção lexical e filtros](evidence/phase22g/consolidation/c1-trace.json) · [26 consultas](evidence/phase22g/consolidation/c1.json)

Em C2, a preparação reproduziu 20 extrações arquivadas apenas após comparar os pedidos completos. A trilha exercitou o processamento nativo da fonte original, o job, a proveniência, a revisão e a indexação em três bancos independentes. A primeira passagem expôs trocas de finalidade por linguagem, perda do segundo termo de uma comparação e confusão entre preferências. A revisão distinguiu finalidade solicitada, escolha e implementação, os equipamentos nomeados e as dimensões de preferência. O reteste reutilizou as mesmas bases e vetores de consulta. [Preparação, índices e primeira recuperação](evidence/phase22g/consolidation/c2-native.json) · [Referências e fatos do reteste](evidence/phase22g/consolidation/c2-retrieval-final.json)

A atualização de RAM requer vínculo com a fonte USER e correspondência única da propriedade, do dispositivo, do valor anterior ativo e do escopo. Em LLM, uma proposta vazia ou que omitiu o valor atual pode aproveitar a correção explícita validada na fonte. O fluxo preserva a política de revisão e o controle de versão. A regressão verificou a recuperação atual e histórica depois de reabrir o banco. As contraprovas incluem dispositivo diferente, valor anterior divergente, hipótese, negação, fala de personagem e citação de manual. A cobertura determinística atual restringe-se às formulações e relações temporais implementadas. [Testes de operação](../crates/lucille-memory/tests/phase22f_operations.rs)

### Respostas e teto do modelo

As nove respostas reais usaram o contexto da **primeira** recuperação, antes dos últimos ajustes de seleção. Em `f2q-11/12/13/14/19` as entregas responderam à pergunta. Em `10` a geração atribuiu ao projeto uma finalidade deduzida de uma atividade, em `16` omitiu a implementação e em `25` declarou falta de informação sobre uma preferência existente. O verificador reteve `18`, cuja geração afirmava que uma escolha de outro projeto ainda não havia sido definida. Um controle posterior aplicou o verificador atualizado ao texto e aos fatos originais de `10` e rejeitou a finalidade sem suporte. As gerações originais permanecem disponíveis com seus vereditos da época. [Pedidos, respostas brutas e resultados](evidence/phase22g/consolidation/c2-answers.json)

O replay de contexto ideal usa gerações arquivadas a 4.096 tokens. A rodada operacional usa 65.536, Nomic em CPU e a trava de 1 GiB. As diferenças de contexto e as falhas observadas na recuperação impedem atribuir todos os erros ao Gemma. A distinção entre escolha e execução continua uma limitação observada em parte das formulações com contexto ideal. O verificador corrigido preservou as 13 respostas úteis e pediu regeneração para as duas incorretas. Não houve nova geração nesse replay. [Oracle e decisões](evidence/phase22g/consolidation/verifier.json)

As nove gerações C2 consumiram 7.036 tokens de entrada e 208 de saída. Foram necessários 14 embeddings novos na preparação e nas consultas. O monitor do supervisor registrou 64 amostras durante a rodada de respostas. O pico observado de 9.275 MiB representa o consumo de toda a GPU. O consumo de tokens do Codex não foi medido. [Medição da rodada](evidence/phase22g/consolidation/answer-runtime.json)

Para aferir todas as onze respostas dependentes com a recuperação corrigida, faltam, no mínimo, cinco primeiras gerações dirigidas: duas consultas sem geração nesta rodada (15 e 17) e três cujo contexto relevante mudou (10, 16 e 25). A confirmação de repetibilidade ou a medição de regenerações custaria chamadas adicionais. A aprovação integral continua pendente.


### Transporte, regressões e encerramento

A aplicação Tauri de QA executou o frontend compilado do projeto com WebView real, perfil full22g e broker exclusivo. Nos três modos a interrupção causou erro visível. Houve um envio USER, um job concluído com uma tentativa de extração e um registro com proveniência por cenário. A observação do recibo ocorreu depois do erro no DOM. Como a conversa só tinha a mensagem de falha, o comando de copiar recusou o texto e nenhum conteúdo foi copiado. Esse resultado mede recuperação do aprendizado depois da falha. A resposta conversacional continuou exibindo o erro original. [GUI, snapshots e recibos](evidence/phase22g/consolidation/gui.json) · [Extrações reais da GUI](evidence/phase22g/consolidation/gui-extraction.json)

Passaram 24 testes Rust em duas suítes e 118 testes TypeScript em sete suítes dirigidas. As 12 chamadas autorizadas foram consumidas: nove respostas C2 e três extrações da GUI, com 8.342 tokens de entrada e 484 de saída no total. Os supervisores registraram a parada solicitada e a identidade dos processos encerrados. As cinco portas de QA ficaram livres. Os quatro bancos sintéticos passaram na integridade SQLite e na verificação de chaves estrangeiras. Os 104 arquivos protegidos coincidiram com os hashes do inventário ao encerrar. [Quota e supervisores](evidence/phase22g/consolidation/quota-closeout.json) · [SQLite](evidence/phase22g/consolidation/sqlite.json) · [Inventário](evidence/phase22g/consolidation/protected.json)

**Decisão.** Manter o PR em rascunho. A recuperação e o transporte atingiram os critérios dirigidos, mas falta demonstrar a qualidade das respostas nas consultas dependentes com a seleção final. Não há recomendação para iniciar a próxima avaliação extensa.

## Fidelidade com a recuperação corrigida — 18 de novembro de 2026

**Decisão: CONTINUAR na 2.2G.** Das cinco primeiras respostas reais, quatro cumpriram a rubrica congelada. A comparação entre dois projetos expôs uma falha na construção do contexto e uma lacuna no verificador. Depois da correção, tanto a regeneração dirigida quanto uma nova primeira resposta atenderam à rubrica. O resultado final cobre cinco consultas dirigidas. A avaliação formal 2.2F continua reprovada.

| Caso | Primeira resposta operacional | Resultado individual |
|---|---|---|
| f2q-10 | Descreveu os roteiros sonoros para um museu tátil | Adequada. Preservou a finalidade de Crivo. |
| f2q-15 | Informou que Lacre foi concluído, com a última placa em braile pronta | Adequada. O estado concluído e o detalhe constavam na fonte. |
| f2q-16 | Identificou TypeScript como escolha e Rust como implementação em andamento | Adequada. Respondeu às duas partes e preservou as modalidades. |
| f2q-17 | Informou o plano de MySQL em Crivo e disse que o registro de Nacar estava indisponível | Incompleta e entregue. Faltou explicar o limite de acesso. Regeneração e novo primeiro passe adequados após a correção. |
| f2q-25 | Atribuiu ao usuário a preferência negativa sobre interfaces carregadas no sistema de reservas | Adequada. Preservou autoria e polaridade. |

Cada pedido real foi comparado às referências canônicas e à versão, escopo, projeto, modalidade e evidência literal de cada fato injetado. Na primeira rodada houve **4/5 adequadas, uma entrega inadequada e zero respostas corretas bloqueadas**. Com as quatro respostas preservadas e o novo primeiro passe do caso corrigido, o resultado dirigido chegou a **5/5**. [Pedidos e textos integrais](evidence/phase22g/fidelity/first-pass.json) · [Análise por átomo da rubrica](evidence/phase22g/fidelity/rubric-review.json)

### Onde a comparação falhou

A recuperação agregada de f2q-17 selecionou corretamente o plano de Crivo. Ao montar as relações do prompt, o sistema também anexou a mesma referência ao banco *escolhido em Nacar* e marcou essa segunda relação como recuperada. A primeira divergência foi **B — contexto**. O verificador aceitou a omissão do motivo de indisponibilidade, uma falha **D**. A geração não revelou nenhum dado de Nacar. A resposta inicial, por si só, não isola uma limitação de capacidade do Gemma.

O seletor nativo passou a resolver qualificadores paralelos de projeto contra o vínculo autorizado do turno. Na relação externa, o contexto agora identifica o projeto fora de escopo, informa indisponibilidade e fornece zero referências. O verificador exige a explicação do limite nessa comparação. Os testes trocaram os nomes e inverteram o vínculo autorizado, além de testar um qualificador genérico. A correção cobre esse padrão de comparação; formulações muito diferentes ainda precisam de evidência.

### Regeneração e custo

No único caso inadequado, reapresentamos o texto bruto original ao verificador real com o contexto corrigido. A violação registrada foi `missing_project_access_boundary`. A aplicação solicitou uma única regeneração, exibiu a resposta verificada e preservou o plano de MySQL. O teste de streaming confirmou que o consumidor recebe apenas a substituição verificada nesse caminho. Num pedido novo independente, o Gemma explicou a mesma restrição já na primeira resposta. A evidência favorece uma falha de integração. Não houve indicação para gastar a oitava chamada num novo controle ideal. O resultado anterior de 13/15 brutas em contexto ideal de 4.096 tokens continua histórico. [Instrução corretiva, segunda geração e verificação](evidence/phase22g/fidelity/regeneration.json) · [Primeiro passe independente](evidence/phase22g/fidelity/corrected-first-pass.json)

| Inferência nova | Chamadas | Tokens de entrada | Tokens de saída |
|---|---:|---:|---:|
| Cinco consultas obrigatórias | 5 | 4.478 | 137 |
| Regeneração dirigida, 1/1 útil | 1 | 1.312 | 48 |
| Reteste independente de f2q-17 | 1 | 966 | 49 |
| **Total** | **7/8** | **6.756** | **234** |

As cinco gerações iniciais levaram entre 1,33 e 2,57 segundos cada no broker local. O Gemma operou com contexto de 65.536 tokens e Nomic em CPU. Nas duas janelas de inferência, o mínimo amostrado de RAM física livre foi de 2.908.344.320 bytes. O pico de memória usada pela GPU inteira foi de 9.299 MiB. O consumo de tokens do Codex não estava disponível. [Duração, recursos, hash do bridge e encerramento](evidence/phase22g/fidelity/run.json)

### Regressões e preservação

Os replays nativos confirmaram C1 obrigatório 21/21, C1 completo 26/26, admissão das 20/20 fontes e seleção exata em 27/27 consultas C2 nos modos fallback e híbrido. Foram usados vetores e extrações arquivados. Passaram 27 testes Rust e 133 TypeScript, incluindo revisão temporal da RAM nos três modos, isolamento, operações sem recibo e replay do oracle anterior. Nesse oracle as 13 corretas foram preservadas e os dois erros retidos. O replay final da resposta antiga de f2q-17 foi bloqueado e as duas respostas corrigidas foram aceitas. [Regressões nativas](evidence/phase22g/fidelity/regressions.json) · [Replays das entregas](evidence/phase22g/fidelity/offline-replays.json)

O transporte GUI permanece no 3/3 medido no checkpoint anterior. Esta rodada exercitou a verificação da resposta no caminho de streaming e preservou o fluxo de transporte de aprendizado. As cinco portas de QA ficaram livres, seis bancos sintéticos passaram nas verificações SQLite e os 104 arquivos protegidos mantiveram seus hashes. A execução partiu de `bfde90e4`; código e evidências estão no commit `2d794853`. A cobertura dirigida permite prosseguir com a fase, sem aprovar a memória operacional nem iniciar uma campanha extensa.
