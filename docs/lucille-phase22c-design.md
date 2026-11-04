# Lucille 2.2C — diagnóstico anterior às alterações

Base local e remota conferida: `97d610e7b508dbf5de9d964be56c41254f739ec7`, PR #10 em rascunho. Nova branch `feature/lucille-phase22c-evidence-authority`. Nenhuma instalação operacional ou alteração de modelos. O diagnóstico usa textos reais já preservados, snapshots tipados explícitos e o executável nativo anterior em outro banco sintético; não é uma nova medição de inferência. As cinco fontes relevantes permaneceram com seus hashes originais durante a reprodução.

| Falha conhecida | Reprodução e causa por estágio |
|---|---|
| Decisão → desenvolvimento | Verificador anterior aceitou o texto problemático sem violações. A gramática detecta “em andamento”, mas perde “em desenvolvimento” sem linguagem na mesma afirmação. Falha de cobertura/representação da afirmação, não de persistência. |
| Reaprendido → voltou a gostar | Texto aceito sem violações. A preferência atual sustenta gostar, mas não a transição pessoal implícita. Falta uma relação tipada separando operação de memória de mudança pessoal. |
| Pasta vazia → projeto sem arquivos | Texto aceito mesmo com projeto cancelado. Os recibos de executor só registram sucesso, sem recurso/resultado/vínculo demonstrável com a entidade. Falha de autoridade e escopo, além da decisão desnecessária de ferramenta. |
| RAM → capacidade excelente | Texto aceito sem violações. A expressão de capacidade/desempenho não é coberta; a quantidade não prova resultados de carga. |
| Troca de banco correta bloqueada | Um predicado de substituição é interpretado como duas afirmações atuais: MySQL e PostgreSQL. O estado anterior do primeiro valor perde sua função na relação. |
| Resumo indireto bloqueado | Vários predicados na mesma sentença compartilham indevidamente modalidade/ator. Escolha de PostgreSQL e plano anterior de JavaScript não autorizam nem contradizem o uso atual de Python. |
| Histórico de projeto cancelado bloqueado | O replay com nome simples passa; a forma real “Jatoba Azul” é comparada só pelo primeiro token com `jatoba-azul`. A validação de nomes precisa aceitar apenas equivalência explícita de separadores, sem fuzzy de entidades. |
| Objetivo atual correto bloqueado | O modificador “atual” vira parte do alvo `atual_learn_...`. Não é contradição canônica. |
| Esquecimento repetido bloqueado | Há recibo concluído `forget_empty`, mas o verificador exige que houvesse registros em qualquer confirmação. Distinguir cumprir pedido idempotente de alegar remoção de registros existentes. |
| Histórico composto perdido | O filtro de nomes trata o modificador “hoje” após “projeto” como nome estrangeiro. A pergunta pede linguagem anterior e estado atual; ambos são barrados antes do ranking, inclusive o cancelamento com cosseno 0,7033. |
| Extração de descrição terminal | O executável original reproduziu `Ineligible V3 source`, sem memória gravada. “projeto continua” é interpretado como projeto estrangeiro; além disso, a cobertura de descrição não aceita essa forma. JSON real do Gemma era válido. Repetir três vezes não altera uma rejeição determinística. A lease foi revogada e o diagnóstico pontual cancelou o job explicitamente. |
| Nota posterior do Agent sem metadados | O `finally` copia mensagens de ferramenta para o Store com apenas papel, texto e IDs de ferramenta, descartando o relatório já aplicado antes da exibição. |
| Resposta no idioma errado | O contrato geral não torna explícito o idioma corrente em cada geração/reparo; memória portuguesa e histórico competem com a pergunta inglesa. Não é perda do fato canônico. |

[Reprodução preservada e hashes](evidence/phase22c/baseline/manifest.json). Os corpora e resultados anteriores permanecem intactos.

## Política de autoridade a implementar

Mensagem USER elegível sustenta o que foi dito agora, sem comprovar gravação. Memória revisada sustenta seu fato versionado; histórico sustenta o estado anterior. Executor sustenta somente a operação, recurso e resultado efetivamente observados. Um vínculo entre diretório e projeto deve ser explícito e indicar cobertura, não ser inferido da conversa. Contexto conversacional pode identificar referentes, sem inventar estado; inferência do modelo nunca cria evidência ou autorização de escrita.

A avaliação de afirmações usará relações tipadas e vereditos separados (sustentada, contradita, insuficiente, fora do escopo, histórica, hipotética, dependente de ferramenta, não relacionada à memória). Reconhecimento linguístico continua limitado e testável; mais expressões regulares não equivalem a prova geral. Investigar reparo localizado com preservação literal das partes confirmadas e uma regeneração real limitada, medindo também disponibilidade e completude. Nenhum aviso genérico será contabilizado como resposta útil.

O desenvolvimento usa regressões conhecidas. A avaliação inédita só começará após novo congelamento e novos conjuntos. A série histórica manterá o mesmo escopo e ordem 20 → 50 → 100, com isolamento por projeto. Builds pesados com modelos desligados; Gemma 65.536 e Nomic CPU/768/2.048; nenhuma alteração administrativa ou instalação operacional.


## Desenvolvimento real v1 e correções seguintes

As seis conversas conhecidas de 2.2B foram reexecutadas com Gemma/Nomic, GUI nativa e perfil 22C: 65 turnos, 65 relatórios únicos, 72 gerações reais. Seis respostas foram retidas, uma regenerada e nenhuma projeção localizada foi aplicada nessa variante. Esses números medem intervenção/entrega; não substituem revisão independente de todas as afirmações. A disponibilidade bruta 59/65 fica abaixo de 95%. Não há aprovação da fase.

A extração da fonte “O projeto continua sendo um catálogo de minerais.” concluiu e a fila dos seis cenários drenou. O híbrido recuperou 38/38 fatos do corpus conhecido, sem falsos positivos, violação de escopo ou exclusões como atuais. Incluiu os dois fatos da pergunta histórica composta, mantendo o limiar 0,6. FTS recuperou 33/38; Nomic isolado 27/38 e permanece reprovado isoladamente. As saídas negativas foram preservadas em `docs/evidence/phase22c/development/`.

A rodada mostrou três bloqueios por idioma, dois por conteúdo indevido (retomada de gosto e ordem histórica SQLite/cancelamento) e um falso bloqueio de predicados coordenados. O resumo correto combinava uso atual de Python, decisão PostgreSQL e plano JavaScript; o verificador aplicava cada verbo a todos os valores na frase. A segmentação agora reconhece coordenação com um novo predicado explícito. Testes positivos e negativos mantêm a rejeição de JavaScript planejado como uso atual.

A diretiva de idioma agora resolve a pergunta USER original antes de sua concatenação com dados portugueses pelos templates estritos. A regeneração inclui o idioma resolvido e relações rejeitadas com seus motivos, sem construir uma resposta substituta. A projeção localizada pode preservar um estado cancelado comprovado solicitado, removendo uma frase independente de ordem histórica não demonstrada; preserva original/remoção/reverificação. A identidade sintética de desenvolvimento também permite ferramentas quando o USER pede observação externa, para que o ensaio positivo não seja desabilitado por sua configuração.

Passaram 126 Rust na implementação nativa, 685 TypeScript após as novas regressões (incluindo os 666 originais), três testes de esquema, typecheck, lint alterado e build estático. Os ajustes seguintes ainda precisam de geração real; nenhum conjunto final inédito foi criado ou executado.


## Desenvolvimento real v2

O reteste focado entregou a retomada como preferência presente e o resumo com Python atual/PostgreSQL decidido/JavaScript planejado. As respostas em inglês passaram em Chat e Code. O objetivo em inglês ainda foi retido: o resultado bruto correto “learning German” não era normalizado para “aprender alemão”. A equivalência lexical limitada `learning→learn` foi adicionada com controle negativo de outro objetivo; isso não muda o estado de conclusão nem autoriza um objetivo distinto.

O Agent executou `file_list(.)` para uma solicitação explícita e limitou a resposta à pasta. Uma consulta a diretório inexistente produziu falha real, sem concluir nada sobre os arquivos do projeto. Em sessões seguintes, respondeu cancelamento e linguagem histórica/estado atual sem usar ferramentas. O ensaio teve notas intermediárias vazias; portanto não comprova preservação de cópia de narração não vazia. Será realizado um pedido explícito de narração. Os dados conhecidos v2 foram preservados com hashes das fontes privadas; somente caminhos do perfil QA foram substituídos, com hashes antes/depois, e mensagens SYSTEM não foram publicadas.

A verificação de pasta vazia agora vincula “pasta atual” ao recurso `.` e apenas ao último resultado de listagem: uma falha posterior não pode usar o resultado vazio anterior. O observador QA ganhou RPC nativo limitado a operações de teste/falha, escopos `phase22c-fault-*` e fontes USER vinculadas, pois CDP não está disponível. Isso pertence apenas ao host isolado, sem executor de scripts geral e sem alteração da aplicação operacional.
