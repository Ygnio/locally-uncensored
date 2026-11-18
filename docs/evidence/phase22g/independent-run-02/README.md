# Avaliação independente da memória · tentativa 02

**Decisão: INCONCLUSIVO.** O Nomic, o Gemma e o host Tauri iniciaram na máquina de QA. Durante a inicialização da interface, a RAM física disponível cruzou o piso preventivo de 1 GiB. O controlador interrompeu a primeira fonte antes de enviá-la. As 40 consultas permanecem sem observação; nenhuma propriedade da memória pôde ser julgada nesta tentativa.

## Execução e causa da interrupção

O pré-teste oficial passou no registro `d9b583033c2a19dc6287a84a0d8ccfd31869c6d6` e os cinco testes determinísticos passaram. Foram conferidos o executor `e690687c453b1d39cacca67063eb1d784e5db9a4`, o código funcional `97c3f31a187bd10e40b74122f15c40fe7500c2be`, o manifesto e o gabarito da âncora original. O controlador foi iniciado uma única vez. A tentativa ocupou exclusivamente o diretório de campanha `independent-v2`. [Registro de início](start.json)

| Marco em 18/11/2026 (UTC) | Evidência |
|---|---|
| 22:27:49 | O controlador registrou o início. |
| 22:28:36 | O host Tauri do perfil sintético i1 abriu após a prontidão dos serviços. |
| 22:28:38 | O driver nativo da interface sinalizou prontidão. |
| 22:28:42 | O marcador persistente de parada registrava `physical RAM floor`. |
| 22:29:07 | O controlador reservou a etapa i1-01. Ao entrar em `dispatch`, encontrou o marcador e encerrou antes de chamar `send_control`. |
| 22:29:10 | O controlador gravou o encerramento com uma etapa interrompida, 67 pendentes e zero tickets. |

A primeira fonte aparece como `interrupted` porque o executor grava o marcador de exposição **antes** de consultar a guarda de despacho. O estado anterior mostra banco ainda não inicializado; o trecho do broker para essa etapa tem zero bytes. Os logs globais de chamadas e erros conservaram os comprimentos registrados antes do início. Não houve relatório do driver para i1-01, envio ao modelo, resposta no DOM ou recibo a adjudicar. Nenhuma etapa recebeu decisão automática. [Estado anterior](i1-01-before.json) · [Reserva da etapa](i1-01-exposure.json) · [Guarda e broker](stop.json) · [Interrupção](interruption.json)

## Cobertura

| Unidade | Previstas | Concluídas | Interrompidas antes do envio | Pendentes no controlador |
|---|---:|---:|---:|---:|
| Fontes preparatórias | 28 | 0 | 1 | 27 |
| Consultas avaliáveis | 40 | 0 | 0 | 40 |
| Todas as etapas | 68 | 0 | 1 | 67 |

Aprendizado, recuperação, atualização, temporalidade, autoria, esquecimento, isolamento e continuidade ficaram com **0 consultas executadas de 5** cada. Por modo, ficaram pendentes 17/17 em Chat, 13/13 em Code e 10/10 em Agent. O ledger preserva os 40 IDs e o arquivo de encerramento, os estados individuais das 68 etapas. [Ledger](ledger.json) · [Encerramento](closure.json)

## Métricas

| Indicador | Contagem segundo a rubrica | Leitura |
|---|---|---|
| Disponibilidade, utilidade, completude e respostas completas fiéis | 0/40 em cada razão sobre inscrições | Zero observações e 40 pendências. |
| Fidelidade das respostas completas entregues | 0/0 | Nenhuma resposta completa entregue. |
| Fidelidade das afirmações | 0/0 | Nenhuma afirmação verificável entregue. |
| Materialização | 0/30 fatos esperados comprovados | Todos os 30 sem observação. |
| Recuperação obrigatória | 0/51 vínculos esperados comprovados | Nenhuma consulta chegou à recuperação. |
| Precisão da recuperação | 0/0 | Nenhuma referência injetada. |
| Falsas aceitações e falsas retenções | 0/0 em cada taxa | Nenhum rascunho avaliado. |
| Segurança da memória | 0 entregas críticas, 0 confirmações prematuras, 0 reintroduções observadas | Zero entregas e operações nas quais avaliar esses eventos. |

O pontuador congelado retornou `incomplete`. As contagens sobre inscritos indicam cobertura e não permitem estimar desempenho. A primeira causa observável é de infraestrutura. A abertura do frontend e o carregamento dos modelos coincidiram no intervalo medido; a telemetria registra memória do sistema e não permite atribuir a queda exclusivamente a um processo. [Pontuação oficial](score.json)

## Recursos, orçamento e integridade

O Gemma 4 12B ficou pronto com contexto de **65.536** e o Nomic em CPU. Em 23 amostras do supervisor do Gemma, o mínimo de RAM física livre foi **1.055.125.504 bytes**. Em 19 amostras do Nomic, o mínimo foi **1.059.889.152 bytes**. Ambos ficaram abaixo de **1.073.741.824 bytes**. O mínimo de margem de memória comprometida observado foi 12.737.556.480 bytes; a VRAM usada no dispositivo inteiro atingiu 9.467 MiB. A janela entre início e encerramento da campanha foi de aproximadamente 81 segundos. [Amostras Gemma](gemma-resources.ndjson) · [Amostras Nomic](nomic-resources.ndjson)

Foram consumidos **0/160 tickets Gemma**, com **0** extrações, gerações, regenerações, passos adicionais, chamadas malsucedidas, embeddings e tokens de entrada ou saída. Não há latência de inferência. [Lista individual de chamadas](calls.json)

Os dois supervisores verificaram a identidade dos processos de modelo no encerramento. O broker registrou encerramento normal. O lançador encerrou o host e o servidor de frontend próprios. As cinco portas ficaram livres, a trava do controlador foi liberada, os 104 hashes protegidos e a âncora conferiram novamente. Apenas o perfil sintético i1 foi criado. Nenhum SQLite de campanha foi inicializado, portanto não houve banco ao qual aplicar os testes de integridade ou chaves estrangeiras. A primeira tentativa arquivada passou na reverificação. As evidências brutas, inclusive os logs dos serviços e as séries de recursos, ficaram preservadas em `independent-v2`. [Auditoria](audit.json) · [Hashes das evidências locais](raw-sha256.json)
