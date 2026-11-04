# Lucille — Fase 2.2

**Resultado parcial. A Fase 2.2 não está aprovada para uso operacional.**

O código implementa aprendizagem autônoma controlada, correções versionadas, esquecimento com proteção contra replay e indexação derivada. Os testes locais e os benchmarks congelados de extração e recuperação V3 passaram nos limites agregados. A inspeção literal da aplicação real encontrou extrapolações nas respostas; esses resultados impedem declarar a fase concluída, mesmo quando a recuperação está correta.

## Repositório e isolamento

Fork: [Ygnio/locally-uncensored](https://github.com/Ygnio/locally-uncensored). Base: [PR #8](https://github.com/Ygnio/locally-uncensored/pull/8), aberto, rascunho e não mesclado; branch `feature/lucille-phase21d-real-quality`, HEAD `4116a9395582adf56c40100109783f67594c45b8`, conferido antes do trabalho e novamente durante a validação.

Branch desta fase: `feature/lucille-phase22-autonomous-memory`. Commits de implementação:

- `20e0fa92c41c727b474b2d762f7d0e726f5e4747`: autonomia opt-in, memória conversacional e indexação derivada; fonte avaliada na V1.
- `81be5575889ae76d6ddbbddef036bbc9f116222f`: correções de negação e relevância; fonte avaliada na V2.
- `80d6cc5d506cc4858c246f7ff1e4068fe8d067a0`: atribuição de ficção e normalização de categorias de revisão; fonte avaliada na V3.

Os conjuntos congelados foram registrados no commit `c522664b`. O HEAD final publicado, o PR empilhado e a auditoria única de CI são identificados na descrição do PR e no rodapé deste relatório local. Nenhum merge, release ou instalação operacional foi realizado.

Todos os dados de ensaio são fictícios. Checkout, perfil, SQLite e WebView são isolados. Portas próprias: 15283/15284/15287/15288/15289. A instalação em Program Files, perfis e conversas reais, launchers e modelos originais em C:\AI, drivers e pagefile não foram alterados. Os processos encerrados pelos helpers passam por verificação de PID, executável e criação; o host também verifica a linha de comando. Aplicativos existentes permaneceram abertos; o usuário passou a assistir a vídeo no Firefox durante a avaliação.

## Arquitetura e comportamento

O Gemma propõe trechos literais. O validador verifica autoria, evidência completa, entidade, propriedade e escopo, e então aplica a política de risco. Confiança do modelo não concede autoridade de gravação. SQLite permanece canônico; embeddings representam apenas versões atuais elegíveis.

Nível A: preferências explícitas suportadas podem ser ativadas automaticamente apenas com política opt-in e prova independente. Nível B: projetos, hardware, objetivos, tarefas, restrições, informações temporárias e ambiguidades continuam em revisão. Nível C: terceiros, ficção atribuída ao usuário, citações, credenciais, permissões, identidade e escopo divergente são rejeitados. Categorias próximas podem ser normalizadas entre categorias que continuam exigindo revisão; essa normalização não cria acesso à promoção A.

Correções inequívocas de preferências têm snapshots, versões e invalidação do índice. Deduplicação exige compatibilidade canônica; RPG tático e estratégia por turnos permanecem preferências distintas. Mudança de computador não vira uma capacidade de RAM inferida.

Esquecimento reconhecido após a persistência do turno exclui versões relacionadas, invalida vetores, revoga trabalhos e mantém tombstones. A interação antiga não pode recriar dados excluídos nem apagar uma preferência nova revisada. Retomada explícita de um valor excluído exige nova origem e revisão. Esse mecanismo não apaga o texto do histórico da conversa nem garante tratamento antes do envio ao provedor.

O trabalhador de índice é serial, prioriza geração, revisa política/cancelamento e valida novamente a versão ao gravar. Sem Nomic, o produto mantém fallback lexical e pode recuperar o índice depois, sem carregar modelos operacionais automaticamente. A política nova permanece desligada por padrão, habilitada somente no QA sintético.

## Modelos e executáveis

| Componente | Configuração real | SHA-256 |
|---|---|---|
| Gemma 4 12B Q4_K_M | contexto 65.536, GPU, cache de prompt CPU/checkpoints desativados | `59656d7494d6376ca97e9e20b64ea2e16cd97f12ec6d47bfccba91cb785b5134` |
| Nomic Embed Text v1.5 F16 | CPU, 768 dimensões, contexto 2.048, mean pooling | `f7af6f66802f4df86eda10fe9bbcfc75c39562bed48ef6ace719a251cf1c2fdb` |
| llama-server | build 11064/a894dae93, 0.4.1-dev, CPU IvyBridge sem AVX2 | `d629481a5c60518afd6679b3fcb9e428a98523628470ed3b88d5261b0df712cb` |
| Bridge V3 | executável nativo ligado ao Store real | `f1b5de5b2682473d14eb4e7ffc74d232e218f58e15ffd9b2ce8c279864f8f5db` |
| Host Tauri V3 | aplicação completa de QA | `87af8e78787d0542da580645b7534f853701794650f3ffa36a8d0ebe48056ded` |

Caminhos locais efetivamente usados:

- `C:\AI\models\Gemma4-12B-QAT-Uncensored-HauhauCS-Balanced-Q4_K_M.gguf`
- `<QA_WORK>\tooling\phase21c-models\nomic-embed-text-v1.5.f16.gguf`
- `<QA_WORK>\tooling\phase21c-llama\llama-server.exe`
- `<QA_WORK>\build\memory\debug\examples\real_validation_bridge.exe`
- `<QA_WORK>\build\tauri\x86_64-pc-windows-msvc\debug\lucille-full-validation.exe`

Pesos reutilizados, sem novo download. Gemma: revisão HauhauCS `ae8045ac2bd216293ca49a3065da2c942dde4b68`; Nomic oficial: `0188c9bf409793f810680a5a431e7b899c46104c`. Os manifestos privados contêm os caminhos exatos; cópias públicas substituem apenas caminhos de usuário/QA. Nenhuma chave, peso, banco ou binário é publicado.

## Regressões locais

| Verificação | Resultado |
|---|---|
| Rust do núcleo | 89 aprovados; inclui os 66 anteriores |
| TypeScript de memória | 563 aprovados em 56 arquivos; preserva os 420 anteriores |
| Paridade/integração com bridge V3 | 23 aprovados em cinco arquivos |
| Typecheck aplicação, QA e configuração | aprovado |
| ESLint dos arquivos alterados | aprovado |
| Paridade de schema Python | três aprovados |
| Build frontend e host Tauri de QA | aprovados |
| Fixtures históricos | 25 hashes iniciais preservados |

Os logs completos serão publicados com caminhos pessoais removidos. Builds pesados ocorreram com os modelos próprios desligados. Esses resultados são locais e não constituem GitHub Actions aprovado.

## Extração, propostas e promoção

Cada resposta real do Gemma foi reaplicada sem reparo aos modos nativos Rules/LLM/Hybrid. O resultado LLM não recebe crédito do fallback híbrido. Corpora e 54 arquivos relevantes foram congelados antes de cada avaliação; versões e falhas anteriores permanecem disponíveis.

| V3: 160 fontes, 115 afirmações esperadas | TP / FP / ausentes | Precisão | Recall | Promoções corretas / indevidas |
|---|---:|---:|---:|---:|
| Rules | 106 / 0 / 9 | 100% | 92,17% | 58 / 0 |
| LLM validado | 98 / 0 / 17 | 100% | 85,22% | 57 / 0 |
| Hybrid | 106 / 0 / 9 | 100% | 92,17% | 58 / 0 |

Zero gravações críticas e zero violações de escopo nesse corpus V3. Precisão das promoções observadas: 100%, dentro do conjunto limitado; isso não certifica linguagem irrestrita.

O modelo bruto ainda propôs afirmações em casos de ficção, negação, autoridade e credenciais. Essas propostas foram rejeitadas pelo validador; não confundir isso com abstinência correta do modelo. A tabela por categoria conserva categorias propostas, divergências de classificação, candidatas retidas e promoções.

Limites importantes de cobertura LLM V3: preferências novas não suportadas 0/5; hardware 3/5; capacidade ambígua 1/5; troca de equipamento 2/5. Comunicação, café, idioma, estratégia por turnos, tarefas, restrições e soluções tiveram todas as afirmações esperadas reconhecidas nos seus pequenos grupos. Planos de projeto reconheceram 10 afirmações em cinco fontes. A precisão agregada não elimina essas lacunas.

O caso V3-035 interrompeu o avaliador porque o bridge Hybrid encerrou com erro ao tentar fallback de uma fonte considerada inelegível. A mesma resposta original foi preservada e reaplicada idempotentemente, sem modificar código, corpus ou resposta. Um helper externo registrou erros equivalentes, detalhes canônicos e estado da fila; rejeições contam como informação ausente quando há gold. A latência desse replay foi excluída do resumo de latências reais. Leases pendentes nesse caminho do bridge são uma limitação explícita da avaliação, não fila concluída com sucesso.

### Falhas anteriores preservadas

- V1 principal: LLM 121/0/20 e híbrido 126/0/15. Apesar dos agregados positivos, o desafio adicional de seis frases revelou cinco promoções incorretas em Rules/Hybrid e quatro em LLM, com quatro e três gravações críticas. A série longa V1 foi interrompida. V1 permanece reprovada.
- V2: LLM 122/0/34, recall 78,21%, abaixo de 80%. O bloqueio indevido de evolução de personagem também fez a preparação da recuperação abortar. Não houve execução das conversas longas V2.
- V3 tem fontes inéditas para extração e recuperação. Os planos longitudinais carregam fatos escritos na V2, mas nunca executados nela; a primeira execução ocorre na V3. Não são apresentados como prova de generalização irrestrita.

## Recuperação Nomic real V3

65 consultas inéditas, 41 referências esperadas, sementes canônicas independentes da extração Gemma. Hardware/projetos foram aprovados explicitamente no QA; preferências elegíveis seguiram a política A. Vetores reais, índice/representação v4, mínimo semântico 0,6 e pesos 0,5/0,5 exclusivos deste perfil de QA; os valores padrão anteriores continuam preservados.

| Modo | TP / FP / ausentes | Precisão do contexto | Recall do contexto | P@3 bruto | R@3 bruto |
|---|---:|---:|---:|---:|---:|
| FTS5 | 29 / 0 / 12 | 100% | 70,73% — reprovado | 27,93% | 72,97% |
| Nomic | 33 / 0 / 8 | 100% | 80,49% | 33,33% | 92,79% |
| Híbrido | 41 / 0 / 0 | 100% | 100% | 33,33% | 92,79% |

P@3 usa denominador fixo três mesmo quando há uma única referência relevante. O ranking pré-filtro do campo híbrido é diagnóstico Nomic, não reconstrução do ranking final fundido. A decisão de injeção é avaliada separadamente. Nenhuma injeção negativa, excluída, expirada ou de escopo errado foi observada. O fallback lexical conserva precisão, mas não atingiu o recall mínimo; não alegar equivalência ao híbrido.

## Interface e respostas

A aplicação Tauri completa usa compositor e controles reais. Chat, Agent e Code compartilham identidade e Store. Cada consulta de prova cria conversa nova: o payload real registra uma única mensagem de usuário e referências com ID/versão/proveniência.

O perfil fictício limita respostas a duas frases, temperatura zero, 384 tokens, pensamento desabilitado, e pede explicitamente preservar planos como planos. Esse ajuste de fixture não é uma garantia do comportamento do perfil operacional.

| Série | Contexto recuperado | Rubrica automática | Inspeção literal | Estado |
|---|---|---|---|---|
| 20 turnos + 10 consultas novas | 10 TP, 0 FP, 0 ausentes | 36/36 | 34/36; uma extrapolação crítica | reprovado no uso da resposta |
| 50 turnos + 8 consultas novas | 8 TP, 0 FP, 0 ausentes | 27/28 | 24/28; duas extrapolações críticas | reprovado no uso da resposta |
| 100 turnos + 10 consultas novas | 10 TP, 0 FP, 0 ausentes | 35/36 | 33/36; uma extrapolação crítica | reprovado no uso da resposta |
| 200 turnos | não executado | — | margem disponível 1105 MiB, abaixo do piso opcional pré-fixado de 1280 MiB | limitação registrada |

No cenário de 20 turnos, “está sendo desenvolvido usando TypeScript” extrapolou uma escolha planejada. Nos 50, ocorreu a mesma extrapolação com JavaScript/SQLite; outra resposta disse que o banco do outro projeto “has not been defined yet”, em vez de afirmar que não havia informação disponível naquele escopo. O primeiro projeto realmente tinha uma escolha de banco, preservada no seu escopo. Não houve vazamento de referência; houve uma afirmação de ausência sem fundamento.

Nos 100 turnos, houve nova afirmação de desenvolvimento em Rust sem evidência de implementação. Outra resposta não utilizou a capacidade revisada de RAM de 50 GB: a fonte literal dizia apenas “Atualizei o computador principal para 50 GB”. O Store vinculou a atualização ao único fato de RAM daquele equipamento e houve aprovação explícita de substituição; porém o payload de contexto fornece conteúdo, sem a propriedade canônica tipada, e o modelo recusou afirmar a capacidade de RAM. Isso é falha de uso/representação, sem número alternativo inventado. A frase ambígua de 39 GB foi rejeitada por incompatibilidade da categoria `decision` proposta pelo modelo; não foi retida como candidata e não sobrescreveu hardware.

O scorer automático não reconheceu todos os sinônimos de implementação e marcou genericamente “preference” numa oferta de ajuda como contaminação. As revisões literais preservam o resultado automático original e registram cada alteração e motivo, incluindo a remoção desse falso alarme. Os limites fixos continuam: ≥90% dos itens aplicáveis **e zero falhas críticas**. Nenhuma média aprova essas respostas.

As 20, 50 e 100 respostas primárias também foram inspecionadas literalmente; a taxa quantitativa acima pertence somente às consultas novas. Uma confirmação “Anotei” foi observada no corpo de 20 turnos. Perguntas em inglês receberam algumas respostas em português; isso é observação adicional, não um critério inventado depois para alterar a rubrica.

## Consolidação, correção e esquecimento

O RPG tático permaneceu em um único registro com três proveniências, versão 3 e duas revisões, incorporando classes/evolução de personagem. Estratégia por turnos permaneceu em outro ID, com duas proveniências. A auditoria não encontrou fingerprints ativos duplicados nem fusões entre essas duas preferências. As correções de linguagem, capacidade e objetivo conservaram os registros anteriores como obsoletos e suas revisões.

O notebook de trabalho do primeiro projeto permaneceu com 18 GB, separado do computador principal e dos equipamentos de 36/10 GB do segundo projeto. O primeiro projeto tem Rust vigente e PostgreSQL planejado; o segundo preserva JavaScript/SQLite planejados. O objetivo de circuitos foi marcado como concluído. Esses são resultados dos dados fictícios, não características do computador real do usuário.

No corpo de 100 turnos, o café foi excluído, mensagens antigas foram repetidas sem alterar o estado canônico e o retorno explícito exigiu revisão. Dois tombstones ficaram preservados no primeiro projeto. Um ensaio funcional suplementar, congelado antes da própria execução e separado dos percentuais independentes, criou uma sessão nova entre exclusão e reaprendizado: zero referências e resposta de informação indisponível. Depois da retomada revisada, uma paráfrase não recuperou a preferência; a consulta específica sobre açúcar a recuperou. Os dois replays, inclusive o pedido antigo de esquecer após o retorno, mantiveram as memórias inalteradas.

SQLite: `integrity_check=ok`, nenhuma violação de FK, nenhum vetor órfão, nenhum fingerprint ativo duplicado. Permaneceram seis vetores físicos de registros obsoletos: dois de desenvolvimento e quatro do cenário final. Nenhum pertence a memória ativa com versão errada, e nenhum foi injetado como atual. O campo original `stale_current_vectors` do audit SQL tem nome impreciso: conta resíduos no espaço de embeddings ativo, não contexto aceito como atual. A revisão complementar conserva o valor e esclarece os filtros. A exclusão lógica está comprovada nesses casos; limpeza física geral e estabilidade de duração ilimitada não estão certificadas.

## Persistência e falhas reais

Após reiniciar somente o host/Vite de QA, as fotografias iniciais de identidade, índice, detalhes e tombstones ficaram literalmente iguais às anteriores. Foram restaurados 31 vínculos de conversa daquele escopo. Das duas consultas novas, RAM foi recuperada e linguagem não. O modelo afirmou que a linguagem ainda não havia sido definida, embora Rust estivesse persistido: falha de recuperação e extrapolação de ausência, não perda do banco. A revisão literal ficou em 4/6 itens e uma falha crítica. Dois probes sustentam contagens, sem conclusão percentual geral de reinícios.

Ao parar somente o Nomic próprio, uma correção de tema ficou canônica, com um vetor pendente. A consulta lexical recuperou o tema escuro. Após reiniciar o Nomic, outro PID com os mesmos pesos/espaço retomou o índice; a consulta em inglês também recuperou o tema. O estágio de recuperação após o serviço ficar pronto levou 8,55 s. Cada consulta teve 4/4 itens aplicáveis, como observação funcional isolada.

Cancelamento: job realmente `running`, requisição Gemma realmente em andamento, cancelamento pelo controle da GUI antes da resposta terminar. A entrada pendente foi removida, a lease revogada e a resposta real tardia reaplicada pelo comando nativo retornou `completed=false`, sem mudança canônica.

Reinício durante extração: duas tentativas tiveram timeout do controlador Node/PowerShell, embora o host reabrisse e cada job terminasse na tentativa 2 com uma única memória. Os registros dessas tentativas permanecem como incompletos. Uma terceira tentativa limitada usou o mesmo lançamento com log em arquivo do coordenador Python. Houve sobreposição real entre encerramento e inferência, retomada na tentativa 2, uma memória/uma proveniência, índice concluído e rejeição explícita da resposta antiga (`completed=false`) sem alteração. Uma sessão nova recuperou português como idioma. O probe extra preserva um erro de codificação em “Qual é” do controlador; o texto de origem da extração, lido do JSON congelado, está correto. Isso não é prova de qualidade geral de entrada em português.

A leitura de leases no ensaio de falha usou SQLite `mode=ro`; somente o Store escreveu no banco. O token antigo ficou apenas na memória do controlador e foi publicado somente seu hash. A causa dos timeouts de lançamento é suspeita de handles/stdio, não diagnóstico global comprovado. Não houve alteração do código nativo, modelo, política ou prompt para fazer a terceira tentativa passar.

A captura nativa do controle remoto falhou com `FrameArrived timed out`. Capturas da própria WebView por CDP funcionaram sem mover mouse/teclado do Firefox. A causa global da falha de captura continua não comprovada. As imagens ficam no ambiente local; os metadados e payloads reais de prova são públicos.

## Recursos e duração

13 épocas de processos próprios, 5852 amostras de telemetria dos modelos, com sobreposição entre Gemma/Nomic; esse total não significa observações independentes. RAM/commit incluem a convivência real com os aplicativos existentes e o Firefox. Os máximos/mínimos são amostrados, sem garantia de cobertura contínua.

| Cenário GUI | Duração observada, incluindo probes | RAM física livre mínima amostrada | Geração GUI p95 | Pagefile CurrentUsage máximo amostrado |
|---|---:|---:|---:|---:|
| 20 turnos | 170,08 s | 797,79 MiB | 4,68 s | 1234 MiB |
| 50 turnos | 326,50 s | 843,83 MiB | 6,97 s | 1219 MiB |
| 100 turnos | 876,91 s | 820,53 MiB | 12,07 s | 1234 MiB |

A latência GUI inclui polling e preparação; não equivale ao tempo puro do núcleo do modelo. Nos chamados do broker de toda a fase, incluindo desenvolvimento e tentativas reprovadas: extração mediana 3,33 s/p95 3,75 s; embeddings 0,092 s/p95 0,182 s; geração primária mediana 3,19 s/p95 9,84 s. Esses agregados não são apresentados como benchmark independente somente da V3.

**Cenário A medido:** mínimo de RAM física livre 622.759.936 bytes (0,58 GiB); menor folga de commit 9.393.922.048 bytes (8,75 GiB); máximo de VRAM usada total da GPU 9876 MiB e mínimo de VRAM livre medido 2240 MiB. VRAM é leitura total da GPU, não atribuição exclusiva ao Gemma. Nas 526 amostras do sampler de sistema, pagefile CurrentUsage variou de 769 a 1343 MiB.

O primeiro sampler falhou na abertura da GUI. Há lacuna aproximada de 19:15–19:46 local para pagefile, não reconstruída; RAM/commit dos modelos têm telemetria separada. `PeakUsage=8007 MiB` já existia antes desta fase e não é pico causado pelo trabalho. O contador da GUI mede o PID nativo, sem atribuir a ele a RAM de todos os descendentes WebView. Amostras com modelos desligados durante builds não são chamadas de medições de inferência.

O SQLite lógico global passou de 376.832 bytes no início do cenário de 20 para 532.480 no final; 643.072 após 50; 712.704 após 100, chegando a 778.240 após os ensaios adicionais. O tamanho global inclui outros escopos sintéticos. Os vetores correntes do primeiro projeto eram 11, sem pendências ao final; o segundo tinha oito. Filas, versões, proveniências e tamanhos por estágio estão registrados. Esse crescimento observado não certifica ausência de vazamentos em duração ilimitada.

**Cenário B projetado:** somar somente a estimativa de mais 2 GiB à RAM física disponível. O mínimo amostrado de A se tornaria aritmeticamente 2,58 GiB. Não é medição nem promessa de latência, VRAM, commit ou pagefile. Nenhum aplicativo existente foi encerrado pelo agente para testar essa hipótese.

Os 200 turnos opcionais foram omitidos com 1105 MiB livres no ponto de decisão, abaixo do piso pré-fixado de 1280 MiB para esse estágio maior. Isso não é aprovação dos 200 turnos.

### Ensaio próximo do limite

Depois de encerrar GUI/Vite e Nomic próprios, o Gemma manteve contexto 65.536. A folga inicial foi 2.985.570.304 bytes, suficiente para a guarda de 2 GiB. O tokenizer contou 62.532 tokens na fonte; a chamada real usou **62.545 tokens de entrada e oito de saída**, sem cache, e devolveu corretamente `JADE-22` e `Rust`. Tempo total 99,44 s. A fonte e o gerador têm hash registrado.

Esse ensaio foi serializado com Gemma sozinho, distinto da convivência GUI+Nomic e distinto de recuperação de memória persistente. Não houve redução do contexto para fazê-lo caber.

## Evidências, encerramento e publicação

Os corpora congelados, resultados negativos V1/V2, extração/recuperação V3 por categoria, diálogos fictícios, proveniência, 34 payloads reais de probes novos, revisões literais e amostras de recursos estão selecionados para publicação. Arquivos de chave, pesos, executáveis e bancos não são publicados. Caminhos pessoais/QA e timezone local são substituídos; hashes originais e públicos são registrados separadamente. Logs em codificação Windows são decodificados estritamente e publicados como UTF-8 com a codificação de origem registrada; não há substituição silenciosa de caracteres.

Ao terminar, as portas próprias 15283/15284/15287/15288/15289 ficaram fechadas. Gemma e Nomic registraram `requested test shutdown`, identidade verificada e exit code 1 de terminação solicitada no Windows. O sampler recebeu seu marcador de parada e seu PID já não existia na verificação. Nenhum processo alheio foi encerrado. As portas operacionais foram apenas consultadas em leitura.

O resultado da tentativa única de GitHub Actions no HEAD final publicado, seguida da auditoria limitada de leitura, consta na descrição do PR e no rodapé local. Nenhuma permissão administrativa é alterada. A auditoria posterior não gera outro commit que altere o HEAD examinado. Testes locais não constituem CI verde.

## Conclusão e limites

**Resultado parcial, sem aprovação operacional.** Houve aprendizagem/promoção controlada de preferências suportadas, preservação de escopo e versões, recuperação híbrida real, exclusão/replay, retomada revisada, persistência e rejeição de respostas tardias comprovadas nos cenários registrados. A avaliação também mostrou extrapolações de plano para implementação, afirmações de ausência sem fundamento, perda de fatos em categorias específicas e paráfrases que não recuperaram dados existentes. A recuperação correta não aprova o uso incorreto na resposta.

Próximo trabalho dentro da frente de memória: melhorar a apresentação de fatos canônicos tipados/temporalidade ao modelo, validar afirmações de progresso/ausência antes de exibi-las, ampliar cobertura de paráfrases/categorias sem abrir promoção indevida e tratar a limpeza física de vetores obsoletos. Novas alterações exigem outra avaliação inédita congelada; os conjuntos públicos atuais são regressões. Nenhuma outra frente foi iniciada e nenhuma autonomia operacional foi habilitada.


## Arquivos de evidência

[Resultados, telemetria e revisões](evidence/phase22/redactions-and-hashes.json), [payloads reais de probes novos](evidence/phase22/v3-fresh-probe-payloads.json), [protocolo prévio](lucille-phase22-acceptance.md) e [reprodução dos ensaios](../qa/phase22-evaluation/README.md). As imagens permanecem no ambiente local; [metadados da captura inicial](evidence/phase22/v3-long20-start-capture.json) e [após retomada](evidence/phase22/v3-restart-verified-capture.json) são públicos.

Os logs brutos conservam espaços e linhas finais do produtor. O check de whitespace passou para fontes/documentos; quatro avisos de whitespace em logs de evidência foram preservados junto das saídas reais.
