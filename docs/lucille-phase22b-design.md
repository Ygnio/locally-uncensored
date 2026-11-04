# Lucille 2.2B — diagnóstico anterior às alterações e arquitetura

Base remota PR #9: `c1840255292724cc07cb3892f964619e39b5584e`, conferida antes da branch `feature/lucille-phase22b-answer-fidelity`. Os 54 hashes da fonte congelada da 2.2 permaneceram iguais no início e ao término do diagnóstico. Os executáveis originais foram preservados. Dados e modelos próprios novos em `lucille-synthetic-real22b`; nenhum perfil operacional utilizado.

## Reprodução e classificação

Sete requisições históricas foram repetidas com Gemma real e produziram novamente as respostas problemáticas. Os testes nativos usaram cópia de leitura do SQLite sintético anterior e novos vetores Nomic reais. Resultados históricos permanecem intactos.

| Caso | Reprodução e etapa responsável |
|---|---|
| Plano TypeScript → execução | contexto textual correto quanto ao desejo, sem contrato tipado/verificação da modalidade; Gemma repete “está sendo desenvolvido usando”. Serialização, interpretação e formulação. |
| JavaScript/SQLite planejados → implementação | repete afirmação de implementação seguida de frase de planejamento. Mesmo defeito, não falha de persistência. |
| Decisão Rust → implementação | decisão de trocar linguagem não comprova execução. Ausência de verificação independente da resposta. |
| Decisão ausente nos resultados → “não definida” | ausência de contexto é promovida indevidamente a fato negativo. Serialização de disponibilidade e formulação. |
| RAM corrigida 50 GB não utilizada | Store liga a correção ao único fato daquele equipamento, com revisão. O payload apresenta só “Atualizei ... para 50 GB”, omitindo propriedade e revisão; o modelo recusa afirmar RAM. Serialização, não perda do dado. |
| Rust após reinício | pontuação lexical 0,667 e cosseno real 0,646 passam pelos limites, mas o filtro de escopo interpreta a palavra “agora” depois de “projeto” como nome de outro projeto. Recuperação/filtro de entidade, não reinício do banco nem defeito do embedding. |
| Paráfrase do café retomado | a memória existe e passa no filtro de entidade/estado. Cobertura lexical 0,333 e cosseno 0,565 não atingem limites 0,4/0,6. Recuperação/representação; o modelo não recebeu o fato. |
| Cinco preferências novas | os trechos propostos são literais, mas a normalização independente não possui mapeamentos para esses conceitos. Extração/cobertura, não persistência. |
| Seis vetores físicos antigos | todos pertencem a registros obsoletos. Estado/versão impedem injeção, mas a transição não recolhe fisicamente esses vetores. Limpeza do índice, sem perda necessária de histórico canônico. |
| Bridge Hybrid com lease presa | erro de validação seguido de fallback também recusado retorna antes de finalizar/revogar a lease. Reprodução: exit 1, nenhum claim persistido, job `running`. Tratamento do erro/bridge e observabilidade da fila. |

## Decisões de implementação

1. Manter SQLite canônico e auditar o schema antes de criar migração. Qualidade/proveniência/histórico existentes permitem derivar fatos contextuais tipados. Separar modalidade original de validade atual/histórica; decisão não é execução e um plano antigo nunca vira fato histórico de execução por simples envelhecimento.
2. Transportar entidade, propriedade, valor, modalidade, escopo/projeto, revisão, ID/versão e evidência curta verificável. Candidatas/conflitos não serão enviados como fatos; eventual limitação de conhecimento não transporta seus valores. Ausência de resultados significa apenas informação não recuperada, não ausência de decisão.
3. Verificação independente e determinística de afirmações tipadas, com limites explícitos. Melhorar a geração com contrato de modalidade e verificar o texto efetivo. Não usar o próprio Gemma como juiz final. Conservar resposta bruta, violações, resposta exibida e custo de eventual regeneração limitada. Uma recusa do verificador conta como perda de utilidade, não como resposta correta automática.
4. Impedir pintura de texto derivado de memória antes da verificação nos três modos. Bufferizar o texto quando o perfil Lucille está vinculado; preservar cancelamento e contabilização de uso. Requisições genéricas sem vínculo mantêm seu comportamento. A adaptação SSE do broker continua identificada como adaptação, sem alegação de streaming nativo.
5. Corrigir interpretação de marcadores de projeto e desenvolver recuperação por intenção/propriedade sustentada, sem afrouxar limites globais nem fazer fuzzy matching de nomes de escopo. Erros de digitação de vocabulário e comparações precisam de testes adversariais e negativos.
6. Ampliar preferências por mapeamentos/extratos literais em domínios seguros, mantendo distinções entre preferências relacionadas. Não transformar “objetiva” automaticamente em “curta”, nem gosto por classes em preferência por RPG tático. Domínios sensíveis e autoridade continuam fora da promoção automática.
7. Limpar vetores de estados/versões inelegíveis em transação, com idempotência e cercas contra escrita tardia; conservar revisões, proveniência e tombstones. Corrigir explicitamente terminação/retry dos caminhos Hybrid recusados.

## Avaliação prevista

Estado intermediário: implementação em desenvolvimento, ainda sem aprovação da avaliação inédita congelada. A aprovação dos testes locais não equivale ao fechamento da fase nem ao sucesso do CI remoto.

O contexto agora deriva fatos tipados do SQLite, com revisão e proveniência, e distingue trilhas de escolha futura e implementação. A leitura de confirmação da resposta revalida versões, exclusões, conflitos e escopo em uma transação; confirmações adicionais limitadas são registradas separadamente dos acertos de recuperação inicial. O verificador tem gramática finita e não constitui prova geral de verdade. Há no máximo uma regeneração real; a segunda falha retém a resposta, contabiliza o custo e representa perda de utilidade.

Os três caminhos da aplicação armazenam o texto até a validação, com registro da resposta bruta e da intervenção. A aprendizagem usa exclusivamente a mensagem original do usuário e pode continuar quando a resposta do assistente foi retida por fidelidade, sem dar autoridade ao conteúdo rejeitado. Os vetores obsoletos são recolhidos atomicamente, preservando histórico e tombstones. O formato do banco permanece na versão 5; a representação e o espaço de embeddings passam à versão 5, sem misturar índices antigos.

Durante o desenvolvimento ocorreram interrupções reais por pressão de memória e uma indisponibilidade do Nomic que impediu o término da indexação. Essas tentativas não são resultados finais. A versão compilada da interface de QA reduziu o servidor auxiliar de aproximadamente 278 MiB para 90 MiB de memória residente. Seu painel Lucille exige o vínculo explícito de QA e um pin válido; o painel não é habilitado em compilações operacionais comuns. Os limites de proteção, modelos, contexto e aplicativos do usuário foram preservados. A inspeção da própria janela por CDP voltou a responder; isso não demonstra correção da falha global de captura de tela.

Preservar 89 Rust, 563 TypeScript e o subconjunto de 23 testes de paridade. Usar os cenários 20/50/100 da 2.2 somente como regressões. Após finalizar o desenvolvimento, congelar novos conjuntos de extração, recuperação, fidelidade e ao menos uma conversa longitudinal. Separar resposta bruta, intervenção, resposta final, incerteza/recusa, perda de informação, latência e tokens. Os limites do roteiro não são reduzidos; qualquer falha crítica impede aprovação.

Gemma permanece configurado em 65.536, Nomic CPU/768/2.048. Inferência serializada, builds pesados com modelos desligados, aplicativos existentes preservados. Os 200 turnos dependem de margem medida; o ensaio longo já feito não será repetido sem motivo. Publicação em PR draft empilhado, sem instalação/merge/release. CI terá tentativa limitada no HEAD final e auditoria de leitura, sem alteração administrativa.
