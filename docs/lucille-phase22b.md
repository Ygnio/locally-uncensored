# Lucille 2.2B — validação em andamento

Esta fase **ainda não está aprovada**. O PR em rascunho é [#10](https://github.com/Ygnio/locally-uncensored/pull/10), sobre a Fase 2.2. Não houve merge, release ou instalação operacional.

## Rodada congelada v1

O código de produção foi encerrado em `d195b65a`; as verificações de integridade dos avaliadores foram fixadas em `e10607fd`, e os conjuntos foram versionados em `f3dd5ac7` antes da execução. O congelamento inclui 4.148 fontes, hashes das compilações, pesos/runtime, representação e índice 5, protocolo 3, schema 5, política, identidade sintética e parâmetros. Os conjuntos anteriores permanecem intactos. As regressões históricas de 20/50/100 serão executadas na versão final corrigida, sem classificá-las como inéditas.

| Medição real v1 | Resultado | Critério |
|---|---:|---:|
| Extração híbrida: precisão | 42 TP, 2 FP; 95,45% | ≥95% |
| Extração híbrida: recall | 42/57; 73,68% | ≥80% |
| Promoção automática | 28 corretas / 29; 96,55% | ≥99% |
| Recuperação híbrida: precisão | 37 TP, 5 FP; 88,10% | ≥90% |
| Recuperação híbrida: recall | 37/40; 92,50% | ≥80% |
| Fidelidade literal final | 36/44 afirmações; 81,82% | ≥95% |
| Ocorrências críticas finais | 5 | 0 |
| Respostas exibidas / avaliadas | 53/57 | Bloqueios separados |
| Respostas retidas | 4 | Perda de utilidade explícita |

A revisão literal cobriu também as 62 gerações brutas, com 59 afirmações avaliadas, das quais 49 sustentadas. O denominador inclui afirmações implícitas de execução, promessas de operações de memória e extrapolações pessoais de desempenho a partir de capacidade de RAM. Conhecimento geral e perguntas sem afirmação de memória ficam fora desse denominador. Os textos, hashes e justificativas individuais serão mantidos nas evidências; não se usou o verificador da aplicação como juiz da sua própria qualidade.

As falhas incluem: preferência limitada a “hoje” promovida como durável; formulações naturais descartadas; equipamentos extras em comparações de dois equipamentos; preferências de outro assunto recuperadas; resposta afirmativa a uma pergunta sobre execução apesar de haver apenas plano; e comandos de esquecimento não reconhecidos quando receberam complementos naturais. O replay não alterou o estado canônico. O problema de exclusão foi a ausência da operação, acompanhada de confirmação indevida pelo modelo e posterior exposição do tema ainda ativo.

Há uma errata explícita no gold de `bx-065`: uma hipótese foi rotulada como `decision`, embora exista o tipo `hypothesis`. O resultado real ainda atribuiu linguagem de projeto ao campo `user.hardware_intention`. O conjunto e os escores originais foram preservados; a errata não transforma a rodada em aprovada.

## Preservação e limites

Todos os dados de teste são fictícios e estão em perfis isolados. Gemma permaneceu com contexto 65.536; Nomic, CPU/768/2.048. Não foram alterados modelos, launchers, perfis operacionais, configuração global ou aplicativos existentes. A janela própria foi inspecionada e capturada por CDP sem entrada no mouse/teclado do sistema. Isso não comprova correção da falha global de captura Remote. O broker adapta respostas não streaming para SSE; streaming nativo não está aprovado.

Antes da v1, passaram 112 testes Rust e uma suíte ampliada de 650 testes da interface/memória, sem falhas ou skips; uma correção posterior de infinitivos acrescentou um teste direcionado. Esses resultados locais não substituem os critérios reais acima. O CI remoto ainda não foi declarado aprovado.

## Próximo fechamento

Corrigir a execução e confirmação nativa dos pedidos de esquecimento, a autoridade de fontes pendentes, temporalidade, normalização e seleção por propriedade. A próxima rodada usará outro congelamento e conjuntos inéditos. A v1 permanecerá como evidência negativa e regressão.
