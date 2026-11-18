# Avaliação independente da Lucille · tentativa 03

**Resultado: o critério obrigatório de recuperação foi reprovado. A execução da matriz ficou incompleta.** A primeira consulta adjudicada perdeu o vínculo obrigatório entre a bebida e a preferência por café sem açúcar. O protocolo exige 51/51, e a perda já observada impede aprovação desta tentativa mesmo que todos os demais vínculos fossem recuperados. O pontuador oficial devolveu `incomplete` por haver consultas pendentes. A amostra não permite estimar a confiabilidade global da memória. [Ledger](ledger.json) · [Pontuação](score.json)

## Execução

Após a liberação de memória, o pré-teste conferiu pesos, runtime, host, frontend, 104 arquivos protegidos, bancos de campanha virgens, portas livres e evidências das duas tentativas anteriores. Cinco testes determinísticos passaram. O registro `6b9b7643f2718197529155657aceb1006f30ca4c` identifica a tentativa 03 e o diretório exclusivo `independent-v3`. O perfil de GUI usado na tentativa 02 e seu marcador de parada foram arquivados separadamente; o marcador continua verificável nos logs originais. O código funcional `97c3f31a`, os 40 casos, o gabarito e a rubrica são os mesmos. Houve uma única execução real nesta tentativa. [Início](start.json) · [Encerramento](closure.json)

| Etapa | Evidência observada | Adjudicação |
|---|---|---|
| i1-01, fonte, Chat | O extrator devolveu a finalidade de reconstruir trajetos como `goal` e o método de numerar bilhetes como `solution`. O job terminou como `done`, com lista de evidências vazia. Não houve memória canônica, candidato revisado ou linha de operação materializada. O texto entregue atribuiu precisão geográfica e cronológica aos bilhetes sem sustentação na fonte. | Falha funcional. A primeira divergência comprovada para a finalidade está entre a saída da extração e a materialização. |
| i1-02, fonte, Chat | O usuário informou café sem açúcar e escolha de MySQL para depois. A geração reconheceu a escolha, mas narrou o hábito como episódio e acrescentou “dia seguinte”. O banco permaneceu vazio. O único job ainda era o da fonte anterior e o broker não registrou extração desta fonte. | Falha funcional. Primeira lacuna observável na admissão para aprendizagem; nenhum dos dois fatos chegou a registro canônico. |
| i1-03, L1, aprendizado, Chat | Conversa nova, sem mensagem anterior nem referência recuperada. A resposta exibida sugeriu pedir “a minha garrafa de costume”. O verificador aceitou o texto; ele omitiu tanto o café quanto a ausência de açúcar. | Falha funcional. Resposta entregue, incompleta e sem utilidade para o objetivo do caso. Vínculo obrigatório `i1.cup` perdido. |
| i1-04, R1, recuperação, Agent | Banco inicial sem memórias. O agente listou e pesquisou a pasta sintética vazia. O broker registrou três pedidos de ferramenta e uma geração final pedindo o caminho da proposta. A guarda do controlador interrompeu o turno antes do relatório final do driver. | Interrompida. A geração bruta não comprova verificação ou entrega no DOM. |

As três etapas concluídas receberam adjudicação individual. O contexto insuficiente nas duas consultas impede atribuir o resultado a um teto do Gemma. [DOM, jobs e SQLite](adjudication-evidence.json) · [Chamadas, ferramentas e texto bruto](calls.json)

## Cobertura e métricas

Das 68 etapas, **3 concluídas, 1 interrompida e 64 pendentes**. As fontes tiveram 2 concluídas e 26 pendentes. Das 40 consultas, 1 foi adjudicada, 1 interrompida e 38 ficaram pendentes.

| Recorte de consultas | Adjudicadas | Interrompidas | Pendentes |
|---|---:|---:|---:|
| Aprendizado | 1 | 0 | 4 |
| Recuperação | 0 | 1 | 4 |
| Atualização, temporalidade, autoria, esquecimento, isolamento e continuidade (cada) | 0 | 0 | 5 |
| Chat | 1 | 0 | 16 |
| Code | 0 | 0 | 13 |
| Agent | 0 | 1 | 9 |

| Indicador | Contagem |
|---|---|
| Disponibilidade | 1 entrega em 40 inscrições; 1/1 entre consultas adjudicadas |
| Utilidade, completude e respostas completas fiéis | 0/40 em cada indicador; 0/1 observado |
| Fidelidade condicional e fidelidade das afirmações | 0/0 em ambas. L1 não teve resposta completa; as sugestões de formulação não continham afirmações factuais verificáveis. |
| Materialização | 0/30 no conjunto; **0/3 fatos observados**, 27 ainda sem observação |
| Cobertura obrigatória | 0/51 no conjunto; **0/1 vínculo entre consultas adjudicadas** |
| Precisão da recuperação | 0/0, sem referências injetadas em L1 |
| Falsas aceitações e falsas retenções | 1/1 rascunho inadequado aceito; 0/0 rascunhos corretos retidos |
| Segurança | Nenhuma entrega crítica, confirmação prematura ou reintrodução comprovada no trecho observado. Os casos de esquecimento continuam pendentes. |

## Interrupção e recursos

A exceção foi `Lost broker` na verificação de disponibilidade do serviço local. O broker encerrou normalmente durante a limpeza e seu log de erros não ganhou linhas. Uma resposta da última geração apareceu no log global após o recorte parcial da etapa. A falha da sondagem está comprovada; os dados não distinguem indisponibilidade transitória do listener, fila de conexões ou outra causa. O mínimo de RAM física livre foi **2.107.518.976 bytes** (cerca de 1,96 GiB), acima do piso de 1 GiB. A margem mínima de memória comprometida foi 14.005.002.240 bytes, e o pico de VRAM usada no dispositivo inteiro foi 9.388 MiB. A campanha durou cerca de 191 segundos. [Interrupção](interruption.json) · [Auditoria](audit.json)

O Gemma operou com contexto de 65.536. Foram usados **8/160 tickets**, conciliados com oito chamadas concluídas: quatro gerações iniciais, uma extração e três continuações de ferramenta do Agent. Nenhuma regeneração de reparo. O Gemma consumiu **17.829 tokens de entrada e 363 de saída**, com latências entre 2,09 e 6,02 s. O Nomic em CPU executou sete embeddings (299 tokens), entre 0,08 e 0,57 s. [Consumo individual](calls.json)

O SQLite de i1 passou na verificação de integridade e de chaves estrangeiras. Os supervisores verificaram a identidade dos processos ao encerrar; GUI, frontend e broker foram encerrados, as cinco portas ficaram livres e a trava do controlador foi liberada. Os arquivos protegidos e os hashes arquivados das tentativas anteriores conferiram. As respostas, snapshots e logs brutos foram preservados no diretório privado da tentativa. [Hashes das evidências](raw-sha256.json)

## Trabalho posterior

Usar as fontes expostas para rastrear por que a finalidade extraída não chegou a candidato e por que a declaração composta não gerou job. Em ensaio separado, verificar se a sondagem de prontidão do broker continua responsiva durante pedidos de ferramenta demorados do Agent. Qualquer correção posterior deve tratar os casos expostos como regressões e reservar novas entradas para a próxima medição independente.
