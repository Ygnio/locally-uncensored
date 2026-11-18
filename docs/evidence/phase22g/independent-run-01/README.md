# Execução independente 2.2G · tentativa 01

**Decisão: INCONCLUSIVO.** A chamada oficial ao controlador ocorreu uma vez. A inicialização do supervisor do Nomic falhou antes de abrir o host Tauri ou despachar a primeira fonte. O controlador registrou 68 etapas pendentes. Nenhum caso foi exposto, julgado ou repetido.

## Ambiente e falha

O checkout, o PR em rascunho, a âncora `8b09ba6647961e9dee731010795b446add1ea2fc`, o produto `97c3f31a187bd10e40b74122f15c40fe7500c2be` e o executor `614a736260f1b5f294897646011fa72fd801e50f` conferiram. O pré-teste oficial verificou os binários, os pesos, o contexto configurado de 65.536, os 104 hashes protegidos, as portas e a ausência de resultados e perfis anteriores. Passaram os cinco testes determinísticos.

A primeira causa observável ocorreu em `scripts/lucille-real-server.py`, linha 136. O supervisor iniciou seu próprio processo Nomic em CPU, capturou uma amostra de recursos e tentou ler `os.environ` sem ter importado `os`. O Python lançou `NameError: name 'os' is not defined`. O bloco de encerramento verificou a identidade do processo e o terminou. O manifesto registra `ready=false`, código de saída 1 e 0,112 s de vida supervisionada. O controlador registrou falha de inicialização e acionou a parada. O pré-teste exercitava a agenda com transporte simulado e o smoke anterior inspecionava a GUI sem modelos; nenhum deles exercitava esta primeira amostra do supervisor real. A constatação anterior de que a infraestrutura estava pronta foi insuficiente nesse ponto. [Registro da interrupção](interruption-public.json) · [Auditoria](audit.json)

## Cobertura e desempenho

| Unidade | Previstas | Executadas | Interrompidas depois do envio | Pendentes |
|---|---:|---:|---:|---:|
| Fontes | 28 | 0 | 0 | 28 |
| Consultas | 40 | 0 | 0 | 40 |
| Etapas totais | 68 | 0 | 0 | 68 |

Aprendizado, recuperação, atualização, temporalidade, autoria, esquecimento, isolamento e continuidade tiveram **0 consultas executadas e 5 pendentes cada**. Em Chat ficaram pendentes 17 de 17, em Code 13 de 13 e em Agent 10 de 10. Não houve conteúdo no DOM, adjudicação de etapa, revisão, recibo de operação, registro recuperado ou resposta verificável.

| Indicador oficial | Resultado e interpretação |
|---|---|
| Disponibilidade, utilidade, completude e respostas completas fiéis | 0 observações para 40 inscrições em cada indicador. As 40 continuam pendentes. |
| Fidelidade das respostas completas e das afirmações | 0/0 em cada população. Taxas indefinidas. |
| Materialização | 0 dos 30 fatos observados. Todos os 30 aguardam exposição das fontes. |
| Recuperação obrigatória | 0 dos 51 vínculos examinados. |
| Precisão, falsas aceitações e falsas retenções | 0/0 em cada indicador. |
| Segurança | Zero eventos observados, com zero entregas e zero operações. |

O pontuador oficial produziu `incomplete`. O ledger indica que as verificações de encerramento terminaram, e preserva todas as consultas como pendentes. Os zeros nas razões de inscrição expressam cobertura observada. A tentativa não permite estimar qualidade ou atribuir falha à memória, ao verificador ou ao Gemma. [Ledger](ledger.json) · [Saída do pontuador](score.json) · [Estado das 68 etapas](closure.json)

Foram reservados **0/160 tickets**, com zero extrações, gerações, regenerações, chamadas malsucedidas ao Gemma, embeddings, tokens de entrada e tokens de saída. Os logs de chamadas e erros do broker tinham os mesmos comprimentos antes e depois da execução. Não há latência de inferência. A única amostra do supervisor registrou 10.902.712.320 bytes de RAM física livre, 8.956.243.968 de 34.312.941.568 bytes de memória comprometida e 409 MiB de VRAM usada no dispositivo inteiro. A falha ocorreu acima do piso preventivo de 1 GiB. [Recursos e reconciliação](audit.json)

## Encerramento e correção posterior

O processo Nomic e seu supervisor deixaram de existir. Gemma, broker e GUI da campanha não chegaram a iniciar. As cinco portas de QA ficaram livres e a trava do controlador foi liberada. Nenhum dos cinco perfis da campanha foi criado; portanto, não havia SQLite de campanha a submeter a `integrity_check` ou `foreign_key_check`. Os 104 arquivos protegidos e a âncora foram revalidados após a parada. Os originais de início, interrupção e encerramento permanecem no diretório exclusivo da tentativa. Os logs brutos de inicialização e de recursos foram retidos ali, e a auditoria registra seus hashes. A versão pública da interrupção omite caminhos pessoais. [Início](start.json) · [Auditoria](audit.json)

Para um ciclo posterior, corrigir a dependência de `os` no supervisor de QA e acrescentar um teste que percorra a primeira amostragem real ou isolada do laço, incluindo o caminho de parada por RAM. Verificar a inicialização e o encerramento dos servidores em área de ensaio sem despachar qualquer entrada da matriz. Registrar uma nova versão do executor, uma âncora explícita e um diretório de tentativa distinto antes de pedir nova execução. O produto, os casos e o gabarito permaneceram nos commits congelados. Esta tentativa se encerra com o diagnóstico de infraestrutura.
