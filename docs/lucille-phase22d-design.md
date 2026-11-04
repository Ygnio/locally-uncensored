# Lucille 2.2D — diagnóstico antes de modificar produção

Base local/remota verificada: `c6e68b4af7d1e136ef6d3b81403a08efb6518e09`. Branch `feature/lucille-phase22d-learning-integrity`. Nenhum modelo foi iniciado nesta reprodução. Executáveis 2.2C preservados e identificados por SHA256; bancos novos com marcador de QA. Somente bancos sintéticos anteriores foram lidos/copied por backup SQLite, sem abrir dados operacionais. Os corpora e resultados 2.2C permanecem intactos.

A reprodução nativa usa respostas reais já preservadas do Gemma e os vetores Nomic originais. É diagnóstico conhecido, não nova inferência nem avaliação inédita. O replay do verificador usa respostas originais em snapshots explicitamente projetados dos registros nativos anteriores. [Resultados e hashes](evidence/phase22d/baseline/manifest.json).

## Correção da atribuição causal anterior

O relatório 2.2C atribuiu a perda de MySQL ao cancelamento da fonte após retenção da resposta. A inspeção do job original encontrou **done, 1 tentativa, sem erro e lease revogada**. O log contém uma extração real do Gemma que indicou corretamente “Planejo usar MySQL no meu projeto”. O replay conclui novamente sem memória. Portanto **a fonte MySQL foi publicada/processada apesar do bloqueio; a causa primária observada é normalização**, e a retenção prejudicou a resposta, não cancelou aquele job. O suporte pré-existente para resposta retida deve ser preservado e testado, sem redesenho transacional baseado nessa atribuição errada. Registros 2.2C não serão reescritos.

## Matriz por estágio

| Fonte USER / caso | Proposta Gemma | Normalização, política e persistência | Recuperação / verificação / exibição | Causa e correção mínima proposta |
|---|---|---|---|---|
| My own preference is coffee without sugar | Evidência literal correta, preference | 0 registros; sem decisão de promoção | Fato indisponível | Autoria nominal e predicado de preferência; representação intermediária, sem reescrever a evidência |
| Entre as bebidas, minha favorita é café | Evidência literal correta, preference | 0 registros | Fato indisponível | Vínculo entre domínio bebida e favorito após topicalização |
| My favorite beverage for myself is tea | Evidência literal correta, preference | 0 registros | Fato indisponível | Modificadores entre entidade/propriedade e cópula |
| Para o meu projeto, a linguagem que decidi usar é Rust | Evidência correta, decision | 0 registros | Escolha ausente | Objeto/valor separados do verbo; decisão não autoriza execução |
| My choice for my project database is SQLite | Evidência correta, decision | 0 registros | Escolha ausente | Predicado nominal de escolha com ator e projeto comprovados |
| Meu computador principal está equipado com 36 GB de RAM | Evidência correta, hardware | 0 registros; requereria revisão | Capacidade ausente | Entidade, relação de posse/equipamento e unidade; negação/modalidade independentes |
| Tenho como objetivo aprender italiano | Evidência correta, goal | 0 registros; requereria revisão | Objetivo ausente | Posse de objetivo sem prefixo rígido; alvo exato e chave compatível |
| Já concluí meu objetivo de aprender francês | Evidência correta, goal | 0 registros | Conclusão ausente | Aspecto de conclusão com advérbio, sem confundir desejo com conclusão |
| Não estou dizendo que meu computador tem 128 GB de RAM | Proposta inclui a frase negada | **1 candidata indevida**; não promovida | Não injetada como ativa | Negação da própria asserção não validada antes de classificar quantidade; rejeitar também candidata contradita |
| Planejo usar MySQL no meu projeto | Extração real correta, decision | Job original e replay **done**, 0 registros | Resposta retida; consultas posteriores sem banco | Normalização compartilhada falha em Planejo; contexto atual também perde a intenção. Formalizar finalização de fonte separada da qualidade do ASSISTANT e testar os três modos |
| Volte a guardar esta preferência: eu gosto de jogos em que posso escolher minha classe | Extração real correta, preference | Após criar/esquecer, replay permanece sem preferência ativa | Sonda posterior gera never-informed e é retida | Intenção de renovação limitada a prefixos literais; reconhecer operação + nova evidência, mantendo tombstone contra replay velho |
| Meu irmão usa qual banco de dados? | Não é etapa de extração | Registro próprio de banco existe | Replay do vetor real injeta SQLite do projeto do USER | Filtro de ator incompleto; sujeito não pode ser substituído por similaridade |
| Você sabe qual é minha data de nascimento? | Não aplicável | Registro próprio de banco existe | Replay injeta banco por “data” | Propriedade data de nascimento confundida com database; intenção tipada precede ranking |
| Qual a memória RAM do meu computador principal? (Nomic ausente) | Não aplicável | RAM 48 ativa e versionada | Replay lexical injeta 0 registros | FTS só indexa conteúdo; correção curta não contém entidade/RAM, e cobertura usa texto bruto. Usar vínculo canônico entidade/propriedade como candidato textual limitado, sem reduzir limiares |
| Qual era o assunto do catálogo que eu estava fazendo? | Não aplicável | Descrição ativa | Replay lexical injeta 0 registros | Intenção descritiva indireta não reconhecida; vincular assunto/descrição ao projeto, sem busca global |
| Ausência correta de GPU | Não aplicável | Há RAM, não GPU | Replay rejeita known_information_not_used | “Alguma memória conhecida” não significa “fato perguntado conhecido”; comparar propriedades/entidades da pergunta |
| Você está desenvolvendo (série 100) | Não aplicável | Descrição de começo + decisões/planos | Replay aceita sem violações | Relação temporal não reconhecida; testar começo genérico, intenção, execução e conclusão separadamente |

## Direção de mudanças

Manter SQLite como único escritor e schema 5 enquanto for suficiente. Acrescentar uma interpretação intermediária pura da evidência (texto/intervalo, ator, entidade, propriedade, valor/unidade, negação, modalidade e origem) que confirme relações antes de produzir Claim. Reusar os mesmos vínculos para leitura/recuperação; manter fronteiras de revisão e promoção. Não aceitar JSON válido como prova e não estender prompts antes de corrigir os estágios comprovadamente responsáveis.

O ciclo atual reserva uma fonte nativa antes de gerar e publica na finalização do turno, com leases e revogação. A retenção do verificador não equivale a cancelamento do usuário; o caso MySQL já prova isso parcialmente. A finalização será explicitada e testada para texto retido inclusive vazio, erro de transporte e interrupção legítima, conservando a mensagem USER original, seu escopo capturado e o fence nativo contra revogação/resposta tardia. Sem promover ASSISTANT/ferramenta, sem confirmação textual como autorização.

Cada mudança será acompanhada de testes específicos e comparação com estes resultados. Novos conjuntos finais serão criados somente após encerrar produção; depois não haverá ajuste por resultados. Builds pesados com modelos desligados, Gemma 65.536, Nomic CPU/768/2.048, cargas serializadas e nenhum aplicativo existente encerrado. CI remoto continua uma investigação limitada, sem alteração administrativa.
