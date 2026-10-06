# AI Codebase Context / Agent Infrastructure
# ROADMAP — Plano Completo de Desenvolvimento

> **Status:** Roadmap oficial inicial do projeto
>
> **Objetivo:** transformar a ideia de AI Codebase Context / Agent Infrastructure em um produto open source real, confiável, local-first, multiplataforma e utilizável por desenvolvedores e agentes de IA.
>
> **Regra central:** o projeto será desenvolvido em fases sequenciais. Uma fase só é considerada concluída quando seus critérios de aceitação forem atendidos. Não adicionar funcionalidades de fases futuras apenas para “adiantar” o projeto.

---

## 0. Como ler e seguir este roadmap

Este arquivo é simultaneamente um roadmap de produto, uma especificação de execução e um conjunto de gates técnicos.

Cada fase possui duas explicações:

- **Explicação simples:** o que deve existir do ponto de vista de quem usa o produto.
- **Explicação técnica:** o que precisa existir internamente para que isso funcione corretamente.

Cada fase também possui:

- objetivo;
- dependências;
- escopo;
- decisões obrigatórias;
- implementação esperada;
- testes;
- entregáveis;
- critérios de aceitação;
- regra de avanço;
- riscos e coisas que não devem ser feitas.

### Regra de execução das fases

A ordem correta é:

`Planejar → Implementar → Testar → Revisar → Documentar → Validar → Avançar`

Nunca fazer:

`Planejar → Implementar 10 features → tentar testar no final`

O projeto deve permanecer executável ao final de toda fase.

### Regra de escopo

Uma feature futura não entra em uma fase anterior apenas porque é “fácil”.

Exemplo: MCP pode ser fácil de adicionar depois do core, mas não deve contaminar a arquitetura do scanner ou do indexador antes de essas camadas estarem estáveis.

### Regra para decisões

Decisões arquiteturais permanentes precisam ser documentadas em `docs/decisions/` como ADRs curtos.

Quando uma decisão ainda for experimental, ela deve ser tratada como tal e não virar API pública antes da validação.

---

# 1. Visão do produto

## 1.1 Definição

O projeto é uma camada local e open source de contexto para codebases e agentes de IA.

Ele transforma um repositório em uma representação estruturada, pesquisável e reutilizável que pode ser consumida por:

- humanos;
- CLIs;
- coding agents;
- IDEs;
- integrações MCP;
- ferramentas de automação;
- outros softwares.

A visão de longo prazo é:

> **Universal Context Layer for Software Agents**

A ideia não é criar outro ChatGPT, outro IDE ou outro agente autônomo. O projeto é uma infraestrutura intermediária que conhece o repositório e fornece o contexto correto no momento correto.

---

# 2. Objetivos do projeto

## 2.1 Objetivo principal

Criar uma ferramenta que permita a um agente de IA entender um codebase sem precisar receber indiscriminadamente todos os arquivos do projeto.

## 2.2 Objetivos de produto

O produto deve buscar:

1. compreensão estrutural do projeto;
2. busca rápida;
3. seleção inteligente de contexto;
4. baixo consumo de contexto;
5. privacidade por processamento local;
6. interoperabilidade;
7. execução simples;
8. experiência excelente no terminal;
9. suporte real a Windows, macOS e Linux;
10. alta qualidade técnica;
11. possibilidade de integração por MCP;
12. possibilidade de evolução para uma infraestrutura reutilizada por outros projetos.

## 2.3 Objetivo de distribuição

O projeto será open source desde o início e será desenvolvido pensando em:

- GitHub;
- GitHub Releases;
- crates.io;
- gerenciadores de pacotes posteriormente;
- documentação pública;
- exemplos reais;
- benchmarks públicos;
- contribuição comunitária.

Stars são uma métrica de interesse e visibilidade, não um requisito funcional. O produto deve ganhar estrelas porque é útil e porque é fácil de compreender, experimentar, recomendar e integrar.

---

# 3. Posicionamento estratégico

## 3.1 O projeto não deve ser “mais um chatbot de código”

O produto não deve competir diretamente tentando ser:

- um clone de ChatGPT;
- um clone de Cursor;
- um clone de Claude Code;
- um agente autônomo completo;
- uma interface genérica para Ollama;
- uma plataforma SaaS de IA.

Ele deve funcionar como infraestrutura.

## 3.2 Mensagem central

Mensagem conceitual:

> **Turn any codebase into AI-ready context.**

Outra formulação possível:

> **Build context once. Query it from anywhere.**

Essas frases podem mudar no lançamento, mas a ideia técnica deve permanecer.

## 3.3 Posicionamento competitivo

Existem ferramentas que já geram contexto, fazem busca semântica, operam como MCP servers ou compilam repositórios para LLMs. Portanto, o projeto não deve afirmar ser “o primeiro” nem presumir que a categoria está vazia.

A diferenciação deve vir da combinação de:

- índice persistente e local;
- arquitetura aberta e modular;
- retrieval híbrido e auditável;
- contexto estruturado por símbolos e relações;
- forte respeito à privacidade;
- CLI simples;
- distribuição por binário único;
- suporte cross-platform real;
- MCP como camada de acesso, não como núcleo inteiro;
- formato de dados e API internos pensados para integração;
- benchmarks reproduzíveis;
- qualidade de documentação;
- capacidade de reutilizar o mesmo índice por diferentes agentes.

A meta não é ter mais features que todos os concorrentes. A meta é construir uma camada fundamental que outros agentes possam consultar.

---

# 4. Decisões técnicas fechadas

Estas decisões devem ser tratadas como baseline do projeto.

## 4.1 Linguagem principal

**Rust.**

Motivos:

- binário único;
- ótimo desempenho para scanner/indexador;
- segurança de memória;
- baixo consumo relativo;
- compilação para Windows/macOS/Linux;
- excelente ecossistema para CLI;
- facilidade de distribuir o executável sem depender de Python ou Node do usuário.

O projeto usará Rust 2024 Edition.

A documentação atual do Rust assume a edição 2024 e Rust 1.97.0 ou posterior; o projeto deve acompanhar o stable e manter um MSRV explicitamente documentado e testado. No início, **MSRV = Rust 1.97.0**, salvo decisão ADR posterior. 

## 4.2 Build system

**Cargo** será o único sistema de build principal.

Não adicionar Node, Python ou outros runtimes como dependências de execução do produto.

Ferramentas auxiliares podem existir durante desenvolvimento, mas o usuário final não deve precisar delas para executar o binário.

## 4.3 Organização do repositório

Usar um **Cargo workspace** com crates pequenos e responsabilidades claras.

Estrutura-base:

```text
project/
├── Cargo.toml
├── Cargo.lock
├── crates/
│   ├── core/
│   ├── scanner/
│   ├── parser/
│   ├── graph/
│   ├── index/
│   ├── retrieval/
│   ├── context/
│   ├── mcp/
│   └── cli/
├── tests/
├── fixtures/
├── examples/
├── docs/
│   └── decisions/
├── benches/
├── .github/
│   └── workflows/
├── README.md
├── ARCHITECTURE.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
├── LICENSE
└── ROADMAP.md
```

O nome do binário definitivo pode ser alterado antes do release público. Até a decisão de branding, o código pode usar um nome interno de trabalho.

## 4.4 CLI

Usar **clap** para parsing dos argumentos e definição dos subcomandos.

O CLI deve ser estruturado como interface estável, não como conjunto de scripts soltos.

## 4.5 Serialização

Usar:

- `serde` para tipos serializáveis;
- JSON para formatos de máquina;
- TOML para configuração do projeto;
- Markdown para exportação humana/LLM.

### Decisão sobre configuração

A configuração do projeto será **TOML**, não YAML.

Motivos:

- ecossistema Rust muito bom;
- sintaxe previsível;
- menor ambiguidade;
- bom encaixe em ferramentas de desenvolvimento;
- fácil versionamento.

Arquivo padrão:

```text
.context.toml
```

Esse arquivo descreve configurações do indexador/retrieval, não armazena o índice inteiro.

## 4.6 Parsing de código

Usar **Tree-sitter**.

Tree-sitter é um parser generator e biblioteca de parsing incremental que fornece árvores sintáticas e atualização eficiente quando o código muda. A biblioteca possui bindings oficiais para Rust. Isso permite que o sistema faça análise estrutural sem executar o código do projeto. 

## 4.7 Ignorados e descoberta de arquivos

Usar uma solução baseada no crate **ignore**, que fornece traversal recursivo respeitando `.gitignore`, globs e filtros de arquivos. 

O walker do projeto deve ter uma camada própria em cima disso para aplicar as políticas adicionais do produto.

## 4.8 Banco de dados local

Usar **SQLite** como armazenamento persistente local.

A pesquisa textual inicial utilizará **FTS5**, que é a extensão de full-text search do SQLite e suporta ranking, prefixos, frases e consultas booleanas. 

O projeto deve usar SQLite de forma embutida, sem servidor externo.

## 4.9 Busca semântica

A busca semântica será opcional e local-first.

A primeira implementação será baseada em **embeddings locais usando fastembed**, ativada separadamente do modo lexical. O fastembed atual oferece geração local via ONNX, cache de modelos e diversos modelos de embedding, incluindo modelos voltados a código. 

Não exigir embeddings para que o produto funcione.

## 4.10 Armazenamento vetorial

Não usar um banco vetorial externo no MVP.

Os vetores serão armazenados localmente de forma que o core consiga operar sem serviço de rede.

A implementação inicial deve priorizar simplicidade e correção:

- SQLite para metadados;
- FTS5 para pesquisa lexical;
- vetores armazenados localmente;
- busca exata em um conjunto candidato pequeno ou moderado;
- ranking híbrido.

ANN/HNSW só será adotado depois de benchmark mostrar necessidade real.

## 4.11 MCP

MCP será uma camada de integração posterior.

O servidor MCP inicial será:

- local;
- read-only;
- preferencialmente via STDIO;
- sem executar shell arbitrário;
- sem modificar arquivos.

O protocolo deve ser implementado conforme a especificação MCP vigente no momento da implementação, sem congelar manualmente uma versão desatualizada.

---

# 5. Comportamento geral do sistema

A arquitetura conceitual será:

```text
Repository
    │
    ▼
Project Discovery
    │
    ▼
Secure Scanner
    │
    ▼
Language Detection
    │
    ▼
Tree-sitter Parsing
    │
    ├── Symbols
    ├── Imports
    ├── Definitions
    └── Structural Relations
    │
    ▼
Chunk Builder
    │
    ▼
Persistent Index
    │
    ├── Metadata
    ├── FTS5
    └── Optional Embeddings
    │
    ▼
Retrieval Engine
    │
    ├── Lexical
    ├── Semantic
    ├── Structural
    └── Path/metadata
    │
    ▼
Context Compiler
    │
    ├── token/size budget
    ├── deduplication
    ├── diversity
    └── provenance
    │
    ├───────────────┬──────────────────┐
    ▼               ▼                  ▼
CLI output       Markdown/JSON       MCP
```

---

# 6. Modelo mental do usuário

O usuário não deve precisar entender:

- embeddings;
- AST;
- FTS5;
- índices;
- ranking;
- grafos;
- tokens.

Ele deve entender apenas:

> “A ferramenta conhece meu projeto e encontra o contexto certo.”

Exemplo conceitual:

```text
$ tool analyze
✓ 2,843 files discovered
✓ 1,102 source files indexed
✓ 14,420 symbols extracted
✓ 8,931 relationships discovered
✓ index ready
```

Depois:

```text
$ tool search "authentication middleware"

1. src/auth/AuthMiddleware.ts
2. src/auth/AuthService.ts
3. src/routes/user.ts
4. src/models/User.ts
```

Depois:

```text
$ tool context "add OAuth login"

Context budget: 30,000 tokens
Selected: 11 files
Estimated usage: 27,430 tokens

[context returned]
```

O produto não deve esconder o fato de que o contexto foi selecionado. Deve mostrar o suficiente para o usuário confiar no resultado.

---

# 7. Comandos públicos planejados

O conjunto inicial será:

```text
init
analyze
index
search
context
map
status
config
mcp
version
help
```

### `init`

Inicializa configuração do projeto.

### `analyze`

Analisa o repositório e apresenta visão geral.

### `index`

Constrói ou atualiza o índice persistente.

### `search`

Pesquisa arquivos, chunks e símbolos.

### `context`

Recebe uma tarefa/pergunta e monta contexto relevante.

### `map`

Exibe o mapa estrutural do projeto.

### `status`

Mostra estado do índice.

### `config`

Exibe e diagnostica a configuração carregada.

### `mcp`

Inicia o servidor MCP local.

### `version`

Exibe versão do produto e build information relevante.

Não adicionar dezenas de comandos no início.

---

# 8. Fase 0 — Fechamento do produto e anti-clone

## Explicação simples

Antes de escrever código, precisamos ter certeza do que o projeto é, quem usa, por que ele existe e qual será sua diferença em relação às ferramentas de contexto que já existem.

## Explicação técnica

Criar o contrato de arquitetura e o conjunto mínimo de invariantes que todas as fases seguintes precisam respeitar.

## Objetivos

- congelar a visão do produto;
- documentar não-objetivos;
- documentar concorrentes;
- decidir o que o MVP não fará;
- definir a arquitetura por camadas;
- definir APIs internas de alto nível;
- definir política de privacidade;
- definir critérios de sucesso.

## Decisões

O core não conhece:

- OpenAI;
- Anthropic;
- Google;
- OpenCode;
- Cursor;
- qualquer modelo específico.

Esses elementos existem somente nos adapters ou no ecossistema de integração.

## Implementação

Criar:

```text
ARCHITECTURE.md
SECURITY.md
CONTRIBUTING.md
CHANGELOG.md
docs/decisions/0001-product-boundaries.md
docs/decisions/0002-rust.md
docs/decisions/0003-sqlite.md
docs/decisions/0004-treesitter.md
docs/decisions/0005-config-toml.md
```

## Critérios de aceitação

A arquitetura precisa responder claramente:

- onde o código é lido;
- onde é parseado;
- onde o índice é armazenado;
- onde a relevância é calculada;
- onde contexto é montado;
- onde MCP entra;
- o que é local;
- o que pode ser remoto;
- quais dados nunca devem ser enviados automaticamente.

## Gate

Não avançar enquanto existirem módulos sem responsabilidade definida.

---

# 9. Fase 1 — Fundação do workspace Rust

## Explicação simples

Criar a “casa” do projeto: compilação, módulos, testes, linting e CI básicos.

## Explicação técnica

Configurar Cargo workspace, dependências compartilhadas, feature flags e CI multiplataforma.

## Estrutura inicial

```text
crates/
├── core/
├── scanner/
├── parser/
├── graph/
├── index/
├── retrieval/
├── context/
├── mcp/
└── cli/
```

No início, alguns crates podem ter pouca lógica. Isso é aceitável; o objetivo é evitar um monólito futuro.

## Qualidade

Configurar:

- `cargo fmt`;
- `cargo clippy`;
- `cargo test`;
- `cargo check`;
- Rust stable;
- MSRV;
- GitHub Actions.

## CI inicial

Matriz:

```text
ubuntu-latest
windows-latest
macos-latest
```

Arquitetura primária de CI: x64.

Compilação ARM64 será adicionada na fase de distribuição.

## Critérios de aceitação

Um projeto vazio deve:

- compilar;
- testar;
- passar clippy;
- passar fmt;
- gerar um binário executável;
- funcionar nos três sistemas do CI.

---

# 10. Fase 2 — Contrato da CLI

## Explicação simples

O usuário deve perceber o produto como uma ferramenta profissional desde o primeiro comando.

## Explicação técnica

Criar a estrutura clap, códigos de saída, tratamento de erros e contrato dos comandos.

## Comportamento

Comandos inválidos devem:

- mostrar erro legível;
- explicar a correção;
- retornar exit code apropriado;
- não produzir stack trace por padrão.

## Erros

Usar uma camada de diagnóstico amigável, preferencialmente com `miette` ou solução equivalente, mantendo mensagens estruturadas internamente.

## Saída

Por padrão, saída humana.

Quando solicitado:

```text
--json
```

produzir saída de máquina estável.

## Regras

Nunca misturar logs de diagnóstico com JSON de resposta.

Quando JSON for solicitado:

- stdout = resultado estruturado;
- stderr = logs/diagnósticos.

## Critérios de aceitação

Os comandos existem como skeleton e têm:

- `--help`;
- `--version`;
- códigos de saída;
- tratamento de erro;
- testes de CLI.

---

# 11. Fase 3 — Descoberta do projeto e configuração

## Explicação simples

A ferramenta precisa saber onde está o projeto e como o usuário quer configurá-la.

## Explicação técnica

Criar descoberta de root, carregamento de `.context.toml`, validação e defaults.

## Descoberta do root

Quando executada dentro de um subdiretório, a ferramenta deve procurar pela raiz do projeto subindo diretórios até encontrar um marcador válido.

Marcadores considerados:

- `.git`;
- `.context.toml`;
- `package.json`;
- `pyproject.toml`;
- `Cargo.toml`;
- outros manifestos conhecidos.

Preferir `.git` como sinal forte quando disponível.

Permitir caminho explícito.

## Exemplo de configuração

```toml
version = 1

[project]
name = "example"

[index]
include_docs = true
include_tests = true

[security]
exclude_secrets = true

[retrieval]
semantic = false
default_limit = 10

[context]
default_budget = 30000
```

O schema precisa ser versionado.

## Critérios

- configuração ausente = usar defaults seguros;
- configuração inválida = erro claro;
- versão desconhecida = falha controlada;
- campos desconhecidos = warning ou erro conforme política definida;
- nunca interpretar configuração arbitrária como código executável.

---

# 12. Fase 4 — Scanner seguro de arquivos

## Explicação simples

A ferramenta deve encontrar os arquivos importantes e ignorar automaticamente lixo, dependências e arquivos que não deveriam entrar no contexto.

## Explicação técnica

Criar uma camada de walking que combine:

1. root discovery;
2. `.gitignore`;
3. ignores adicionais;
4. detecção de binário;
5. limites de tamanho;
6. exclusões de diretórios conhecidos;
7. heurísticas de conteúdo gerado;
8. política de segurança.

## Exclusões padrão

Ignorar, quando encontrados:

```text
.git/
node_modules/
target/
dist/
build/
out/
coverage/
.cache/
venv/
.venv/
__pycache__/
```

Também ignorar artefatos binários.

Esses defaults podem ser configuráveis.

## `.gitignore`

A ferramenta deve respeitar `.gitignore` por padrão, mas precisa diferenciar:

- ignorado por Git;
- ignorado pelo Enviro/Context tool;
- incluído explicitamente pelo usuário.

A razão da exclusão deve estar disponível em modo diagnóstico.

## Arquivos especiais

Arquivos como lockfiles podem ser indexados como metadata de baixa prioridade, em vez de completamente ignorados.

## Critérios de aceitação

Em um repositório fixture contendo:

- source;
- docs;
- node_modules;
- arquivos binários;
- arquivos ignorados;
- arquivos grandes;
- arquivos gerados;

somente os itens permitidos aparecem como candidatos.

---

# 13. Fase 5 — Segurança e classificação de conteúdo inicial

## Explicação simples

O produto jamais deve mandar segredos para uma IA por acidente.

## Explicação técnica

Criar uma política de classificação de conteúdo antes do indexador armazenar dados.

## Categorias

### Permitido

Código normal, documentação, testes e configurações não sensíveis.

### Sensível

Arquivos possivelmente contendo:

- `.env`;
- chaves privadas;
- tokens;
- credenciais;
- certificados privados;
- arquivos de secrets.

### Binário

Executáveis, imagens, vídeos, arquivos compactados etc.

### Gerado

Artefatos identificados como gerados.

## Princípio

O sistema deve ser conservador por padrão.

## Segredos

Implementar detecção por:

- nome de arquivo;
- extensão;
- padrões de alta confiança;
- conteúdo.

Não implementar uma “IA de segurança” no MVP.

## Política

Segredos suspeitos:

- não entram no contexto padrão;
- são mascarados nos relatórios quando necessário;
- aparecem como excluídos no diagnóstico;
- só podem ser incluídos explicitamente mediante configuração/flag futura.

## Critérios

Criar fixtures com secrets falsos e garantir que:

- não aparecem em export padrão;
- não aparecem em output de debug normal;
- não são enviados para adapters futuros.

---

# 14. Fase 6 — Detecção de linguagem e metadados

## Explicação simples

O sistema precisa saber o que está lendo.

## Explicação técnica

Criar um registry de linguagens e tipos de arquivo.

## Linguagens prioritárias

Primeiro suporte estrutural:

1. TypeScript;
2. JavaScript;
3. Python;
4. Rust.

Segunda onda:

5. Go;
6. Java;
7. C#;
8. C/C++;
9. Ruby;
10. PHP.

A linguagem não suportada ainda pode ter indexação textual.

## Metadata por arquivo

Guardar:

- path relativo;
- tamanho;
- extensão;
- linguagem;
- hash;
- número de linhas;
- status ignorado/incluído;
- timestamp relevante somente quando necessário;
- detector de conteúdo;
- parser disponível ou não.

## Critérios

Todos os fixtures devem produzir metadata consistente.

---

# 15. Fase 7 — Parsing estrutural com Tree-sitter

## Explicação simples

Agora a ferramenta deixa de apenas “ler texto” e começa a entender que um arquivo contém funções, classes, imports, tipos e outras estruturas.

## Explicação técnica

Criar uma abstraction `LanguageParser` que permita adicionar grammars sem modificar o core.

## Primeiras entidades

Extrair:

- funções;
- métodos;
- classes;
- structs;
- interfaces;
- enums;
- tipos;
- constantes relevantes;
- imports;
- exports;
- namespaces/modules quando disponíveis.

## Modelo de símbolo

Conceitualmente:

```text
Symbol
├── id
├── name
├── kind
├── file_id
├── start_line
├── end_line
├── visibility
├── parent_symbol
├── signature
└── language
```

## Relações

Começar por:

```text
imports
exports
calls/uses (quando confiável)
contains
references
```

Não tentar inferir chamadas com 100% de precisão entre linguagens.

## Princípio

Precisão > quantidade de relações.

Quando uma relação não puder ser determinada com confiança, ela não deve ser inventada.

## Critérios

Fixtures de cada linguagem devem verificar:

- número de símbolos;
- posição correta;
- nomes;
- imports;
- hierarquia;
- tolerância a erros sintáticos.

Tree-sitter deve ser usado para permitir análise mesmo em código parcialmente quebrado.

---

# 16. Fase 8 — Mapa do codebase e grafo estrutural

## Explicação simples

A ferramenta precisa descobrir quais partes do projeto estão conectadas.

## Explicação técnica

Construir um grafo direcionado:

```text
File → File
Symbol → Symbol
```

As arestas devem possuir tipos.

Exemplos:

```text
IMPORTS
EXPORTS
REFERENCES
CALLS
CONTAINS
```

## Graus de confiança

Cada relação poderá possuir:

```text
confidence: high | medium | low
```

## Métricas iniciais

Calcular:

- fan-in;
- fan-out;
- grau;
- arquivos centrais;
- entry points conhecidos.

Evitar algoritmos de centralidade caros antes de existir necessidade.

## Entry points

Heurísticas:

- `main`;
- `index`;
- CLI entrypoints;
- application startup;
- scripts conhecidos;
- routes/controllers;
- arquivos referenciados frequentemente.

## Critérios

O comando `map` deve conseguir representar um projeto fixture de maneira útil.

---

# 17. Fase 9 — Modelo de chunks e fingerprints

## Explicação simples

A IA não precisa receber arquivos gigantes inteiros. O sistema deve conseguir dividir o projeto em unidades úteis.

## Explicação técnica

A unidade principal de retrieval será o **Context Chunk**.

## Tipos

```text
file_header
symbol
section
text_window
configuration
documentation
```

## Estratégia para código

Preferir:

```text
arquivo
→ imports/header
→ classes
→ funções/métodos
→ outras estruturas
```

Em vez de cortar arbitrariamente cada 1.000 caracteres.

## Fallback

Arquivos sem parser:

- chunk por linhas;
- janela configurável;
- overlap mínimo;
- preservar linhas e paths.

## Metadata

Cada chunk precisa ter:

- id;
- file_id;
- start_line;
- end_line;
- content hash;
- language;
- symbol id opcional;
- token/size estimate;
- embedding status;
- source priority.

## Fingerprint

A identidade lógica do chunk deve mudar quando seu conteúdo muda.

Isso será essencial para incremental indexing.

---

# 18. Fase 10 — Persistência local e índice SQLite

## Explicação simples

A análise não deve ser refeita do zero toda vez.

## Explicação técnica

Criar banco SQLite local com schema inicial versionado.

## Local do índice

**Padrão:** diretório de cache do usuário, fora do repositório.

Motivos:

- não poluir Git;
- não precisar de `.gitignore` para o índice;
- facilitar múltiplos projetos;
- evitar commits acidentais;
- separar dados derivados de código-fonte.

O diretório deve ser determinado por plataforma com uma abstraction.

## Identidade do repositório

Usar uma chave estável derivada de:

- caminho canônico;
- metadata do projeto;
- identificação Git quando disponível.

## Tabelas conceituais

```text
projects
files
symbols
relations
chunks
embeddings
index_runs
metadata
```

## Migrações

O schema do banco precisa possuir versão.

Nunca modificar schema silenciosamente em versões públicas.

## Critérios

Depois de fechar o terminal e abrir novamente:

- índice continua disponível;
- status é recuperável;
- busca continua funcionando;
- versão do schema é validada.

---

# 19. Fase 11 — Busca lexical com FTS5

## Explicação simples

Agora a ferramenta realmente responde perguntas simples sobre onde algo está no projeto.

## Explicação técnica

Criar índice FTS5 para chunks e/ou documentos.

Campos principais:

```text
path
symbol
content
language
```

## Ranking

Usar o ranking disponibilizado pelo FTS5 como um sinal, não como score final absoluto.

## Recursos iniciais

- termos;
- frases;
- prefixos;
- filtros por path;
- filtros por linguagem;
- limite de resultados.

## Resultado

Cada resultado precisa apresentar:

- path;
- linhas;
- símbolo;
- score lexical;
- trecho relevante;
- razões básicas de correspondência.

## Critérios

Consultas como:

```text
authentication
payment service
database connection
user middleware
```

devem retornar arquivos relevantes em fixtures reais.

---

# 20. Fase 12 — Context Compiler v1

## Explicação simples

Agora o projeto deixa de apenas pesquisar e passa a construir um pacote de contexto que uma IA consegue usar.

## Explicação técnica

Criar um `ContextCompiler` que recebe:

```text
query/task
budget
retrieval profile
filters
```

E produz:

```text
ContextPackage
```

## ContextPackage

Deve conter:

- pergunta/tarefa;
- overview do projeto;
- mapa relevante;
- arquivos selecionados;
- chunks selecionados;
- paths;
- linhas;
- símbolos;
- relações importantes;
- estimativa de tamanho;
- razões de seleção;
- avisos de conteúdo excluído.

## Orçamento

O usuário poderá definir:

```text
--budget 30000
```

O número é uma estimativa de contexto, não uma garantia universal de tokens de um modelo específico.

## Estimador

No começo usar estimador determinístico baseado em caracteres/estrutura.

Depois permitir tokenizers específicos em adapters.

## Seleção

Algoritmo v1:

1. reunir candidatos;
2. normalizar scores;
3. ordenar por relevância;
4. remover duplicados;
5. respeitar orçamento;
6. aplicar diversidade mínima;
7. adicionar metadata de proveniência.

## Critérios

Nenhum contexto exportado pode ultrapassar o orçamento configurado de forma significativa.

A ferramenta deve informar quando o orçamento foi insuficiente.

---

# 21. Fase 13 — Hierarquia de contexto e compressão estrutural

## Explicação simples

Nem todo arquivo merece ser enviado na íntegra.

## Explicação técnica

Criar níveis de representação.

Proposta:

```text
Tier 0 — apenas path
Tier 1 — metadata/summary estrutural
Tier 2 — símbolos e assinaturas
Tier 3 — trecho relevante
Tier 4 — source completo
```

## Aplicação

Arquivos altamente relevantes podem aparecer em source completo.

Arquivos de suporte podem aparecer apenas como símbolos.

Arquivos periféricos podem aparecer apenas como path/metadata.

## Objetivo

Reduzir ruído sem destruir relações importantes.

## Regra

Nunca resumir silenciosamente e fazer parecer que o conteúdo completo foi analisado.

O output deve indicar o nível usado.

---

# 22. Fase 14 — Retrieval híbrido

## Explicação simples

Busca por palavra ajuda, mas não encontra tudo. Agora o sistema combina diferentes sinais.

## Explicação técnica

Score conceitual:

```text
final_score =
    lexical_score
  + semantic_score
  + structural_score
  + path_score
  + symbol_score
  + recency_score
```

Os pesos devem ser configuráveis internamente e versionados.

Baseline inicial sugerido:

```text
lexical      0.30
semantic     0.30
structural   0.20
symbol       0.10
path         0.05
recency      0.05
```

Quando semantic search estiver desabilitado, os pesos devem ser renormalizados.

## Structural score

Deve considerar, quando disponível:

- fan-in;
- proximidade de entry point;
- relação com resultados principais;
- símbolo diretamente correspondente;
- dependência de arquivo já selecionado.

## Diversidade

Aplicar um mecanismo equivalente a MMR ou heurística de diversidade para evitar selecionar dez chunks quase iguais do mesmo arquivo sem necessidade.

## Auditabilidade

Cada resultado deve conseguir responder:

> “Por que este resultado foi selecionado?”

Exemplo:

```text
score: 0.87
reasons:
- lexical match
- referenced by AuthService
- contains matching symbol
- near entry point
```

Isso é importante para depuração e confiança.

---

# 23. Fase 15 — Embeddings locais

## Explicação simples

Agora o sistema pode encontrar trechos semanticamente relacionados mesmo quando a consulta não usa as mesmas palavras do código.

## Explicação técnica

Adicionar backend de embedding opcional.

O backend inicial será local e baseado em fastembed.

## Requisitos

- modelo baixado somente quando semantic mode for ativado;
- cache local;
- nenhuma API key;
- nenhuma chamada remota obrigatória;
- possibilidade de trocar o modelo no futuro.

## Interface

Criar uma trait semelhante a:

```text
EmbeddingProvider
├── embed_documents
├── embed_query
├── dimensions
└── model_id
```

## Cache

Embedding deve ser invalidado quando:

- conteúdo do chunk mudar;
- modelo mudar;
- versão do algoritmo mudar.

## Primeiro modelo

Começar com um modelo pequeno e adequado a retrieval local. O modelo exato deve ser escolhido por benchmark e licença no momento da implementação.

## Critérios

O semantic mode deve:

- funcionar sem serviço externo;
- sobreviver a reinicialização;
- reutilizar embeddings não alterados;
- conseguir ser desligado totalmente.

---

# 24. Fase 16 — Retrieval contextual avançado

## Explicação simples

A busca passa a responder à tarefa, não apenas à palavra pesquisada.

## Explicação técnica

Implementar expansão de contexto por grafo.

Exemplo:

```text
resultado principal
    ↓
imports
    ↓
implementação
    ↓
tipos relacionados
    ↓
testes
```

## Profundidade

Permitir:

```text
--depth 0
--depth 1
--depth 2
```

Depth padrão = 1.

## Seleção

Não incluir automaticamente todo o subgrafo. Cada relação adicionada precisa disputar o orçamento.

## Priorização

Preferir:

1. implementação diretamente relacionada;
2. tipos necessários para compreender a implementação;
3. dependências internas críticas;
4. testes relevantes;
5. documentação de comportamento.

---

# 25. Fase 17 — Exportação padrão de contexto

## Explicação simples

O usuário pode gerar contexto que outra IA consegue ler.

## Formatos

### Markdown

Formato principal para humanos e agentes.

### JSON

Formato principal para integração de software.

### Compact text

Formato opcional para copiar/colar rapidamente.

## Estrutura Markdown

Proposta:

```markdown
# Project Context

## Task
...

## Project Overview
...

## Architecture Map
...

## Relevant Files
...

## Relevant Symbols
...

## Context
...

## Selection Rationale
...

## Exclusions
...
```

## Proveniência

Cada trecho deve possuir path e linhas.

Exemplo:

```text
src/auth/AuthService.ts:42-118
```

## Critérios

O export precisa ser:

- legível;
- determinístico quando a entrada for a mesma;
- rastreável;
- seguro;
- compatível com o output JSON correspondente.

---

# 26. Fase 18 — Context API interna

## Explicação simples

Antes de criar várias integrações, o sistema precisa possuir uma API interna estável para perguntar coisas sobre o projeto.

## Explicação técnica

Definir interfaces de alto nível:

```text
ProjectService
IndexService
SearchService
ContextService
SymbolService
MapService
```

Exemplo conceitual:

```text
Search(query)
GetSymbol(symbol)
GetFile(path)
GetMap(depth)
BuildContext(task, budget)
GetStatus()
```

Essa API não precisa ser HTTP.

Ela deve ser interna e reutilizável por:

- CLI;
- MCP;
- testes;
- futuros adapters.

## Critérios

CLI e MCP não devem duplicar a lógica do retrieval.

---

# 27. Fase 19 — MCP Server

## Explicação simples

Agora agentes compatíveis poderão consultar o índice diretamente.

## Explicação técnica

Criar um servidor MCP local via STDIO.

## Ferramentas iniciais

### `search_codebase`

Recebe:

- query;
- limit;
- filtros opcionais.

### `get_file_context`

Recebe:

- path;
- range opcional;
- nível de detalhe.

### `get_symbol`

Recebe:

- nome;
- filtros opcionais.

### `get_project_map`

Recebe:

- depth;
- filtros.

### `get_context`

Recebe:

- task;
- budget;
- retrieval mode.

### `get_index_status`

Retorna:

- estado;
- contagem de arquivos;
- chunks;
- symbols;
- última atualização;
- semantic status.

## Recursos opcionais

Expor resources conceituais como:

```text
codebase://overview
codebase://map
codebase://status
```

## Segurança

O MCP inicial deve ser **read-only**.

Não adicionar:

- shell execution;
- write file;
- delete file;
- arbitrary network calls.

## Critérios

Um cliente MCP deve conseguir:

1. descobrir o servidor;
2. consultar status;
3. fazer uma busca;
4. receber paths e linhas;
5. pedir contexto.

---

# 28. Fase 20 — Integração com coding agents

## Explicação simples

A ferramenta começa a trabalhar com o ecossistema de agentes sem depender de nenhum agente específico.

## Estratégia

Primeiro oferecer interfaces abertas:

- MCP;
- JSON;
- Markdown;
- CLI exit codes;
- automação por shell.

Depois criar documentação/integrations para:

- OpenCode;
- Claude Code;
- Cursor;
- Gemini CLI;
- VS Code;
- outros clientes MCP compatíveis.

## Regra

Integrações não podem duplicar o core.

Um adapter só converte:

```text
Context API → formato do consumidor
```

## Critérios

Cada integração documentada precisa ser reproduzível e testada no nível suportado.

Não dizer “integração nativa” quando for apenas consumo por MCP.

---

# 29. Fase 21 — Incremental indexing

## Explicação simples

Depois da primeira análise, alterações pequenas não devem obrigar a ferramenta a reprocessar tudo.

## Explicação técnica

Usar fingerprint por arquivo e chunk.

Workflow:

```text
scan
 ↓
compare hashes
 ↓
unchanged → reuse
changed   → parse again
removed   → delete
added     → index
```

## Git awareness

Opcionalmente utilizar Git para obter um conjunto de candidatos alterados.

Não tornar Git obrigatório.

## Critérios

Alterar um único arquivo deve causar reprocessamento somente do necessário.

---

# 30. Fase 22 — Watch mode

## Explicação simples

A ferramenta consegue manter o contexto atualizado enquanto o desenvolvedor programa.

## Explicação técnica

Adicionar filesystem watcher.

Comportamento:

```text
file changed
    ↓
debounce
    ↓
reindex affected nodes
    ↓
update FTS
    ↓
update embeddings when enabled
```

## Regras

- debounce configurável;
- coalescer alterações;
- não bloquear o terminal desnecessariamente;
- tratamento de erros robusto;
- não entrar em loop se a própria ferramenta criar arquivos temporários.

## Critérios

Editar um arquivo deve atualizar resultados após o período de debounce sem reconstruir todo o índice.

---

# 31. Fase 23 — Performance e escalabilidade

## Explicação simples

O produto precisa funcionar em projetos pequenos e grandes.

## Metas de engenharia

Não usar metas irreais como garantia universal. O desempenho deve ser medido por benchmark reproduzível.

## Benchmarks

Criar datasets em pelo menos quatro tamanhos:

```text
small
medium
large
very-large
```

Medir:

- descoberta;
- parsing;
- indexação inicial;
- indexação incremental;
- busca lexical;
- busca híbrida;
- geração de contexto;
- memória;
- tamanho do índice.

## Profiling

Identificar os maiores gargalos antes de otimizar.

## Paralelismo

Usar paralelismo controlado durante scan/parse/embed.

Não criar centenas de threads sem necessidade.

## Objetivo

A ferramenta deve permanecer responsiva e previsível.

---

# 32. Fase 24 — Qualidade do ranking

## Explicação simples

O sistema precisa provar que está encontrando contexto útil, não apenas produzir resultados bonitos.

## Explicação técnica

Criar dataset de consultas com resultados esperados.

Exemplo:

```text
Question: Where is authentication implemented?
Expected:
- AuthService
- AuthMiddleware
- auth route
```

## Métricas

Avaliar:

- Precision@K;
- Recall@K;
- MRR;
- cobertura de símbolo;
- relevância do contexto final;
- redução de tokens.

## Benchmark de contexto

Comparar:

```text
raw repository
vs
lexical retrieval
vs
hybrid retrieval
vs
hybrid + graph
```

## Regra

Não modificar pesos de ranking só porque uma demo parece melhor. Mudanças devem ser acompanhadas por benchmark.

---

# 33. Fase 25 — Privacidade e segurança avançadas

## Explicação simples

Chegou a hora de garantir que “local-first” seja uma propriedade real e auditável.

## Checklist técnico

Verificar:

- nenhuma conexão remota inesperada;
- nenhum upload automático;
- nenhum telemetry obrigatório;
- nenhum secret exportado;
- nenhum log contendo conteúdo sensível;
- nenhum arquivo arbitrário executado durante indexação;
- MCP sem write/shell por padrão.

## Telemetria

**Não haverá telemetria obrigatória.**

Uma telemetria opcional futura, se algum dia existir, deve ser:

- desligada por padrão;
- explicitamente documentada;
- mínima;
- sem conteúdo de código;
- sem paths privados;
- sem query text por padrão.

## Auditoria

Documentar:

- onde dados são armazenados;
- como limpar o cache;
- como verificar comportamento local;
- quais integrações podem acessar conteúdo.

---

# 34. Fase 26 — Testes cross-platform

## Explicação simples

Windows não pode ser “suporte futuro”. Ele precisa realmente funcionar.

## Sistemas

Prioridade:

1. Windows 11 x64;
2. Ubuntu Linux x64;
3. macOS Apple Silicon;
4. macOS Intel quando viável;
5. Linux ARM64;
6. Windows ARM64 posteriormente.

## Testes

Cada plataforma precisa cobrir:

- path handling;
- file walking;
- cache path;
- SQLite;
- Unicode;
- symlinks;
- line endings;
- subprocessos somente quando estritamente necessários;
- CLI;
- export.

## Critérios

Nenhum código específico de plataforma deve vazar para o core sem necessidade.

Criar uma abstraction `Platform` para diferenças reais.

---

# 35. Fase 27 — Distribuição como binário

## Explicação simples

O usuário deve poder baixar e executar o projeto sem instalar Python, Node ou banco de dados.

## Artefatos

Gerar releases para:

```text
Windows x64
Linux x64
macOS ARM64
macOS x64
Linux ARM64
```

Windows ARM64 e outras arquiteturas podem entrar depois.

## Conteúdo de release

Cada artefato deve possuir:

- nome previsível;
- checksum SHA-256;
- versão;
- changelog;
- instruções.

## GitHub Actions

Pipeline de release deve:

1. build;
2. test;
3. package;
4. checksum;
5. publish artifacts;
6. criar release.

## Regras

Nunca publicar binário que não tenha passado pelos testes correspondentes.

---

# 36. Fase 28 — Instalação por package managers

## Explicação simples

Depois que o binário estiver estável, instalar deve ficar ainda mais fácil.

## Ordem sugerida

1. GitHub Releases;
2. crates.io;
3. Homebrew;
4. winget;
5. Scoop;
6. outros gerenciadores conforme demanda.

## Regra

A distribuição oficial primária sempre deverá existir independentemente de package managers de terceiros.

---

# 37. Fase 29 — README de alto impacto

## Explicação simples

O README precisa fazer uma pessoa entender o produto antes de ela sair da página.

## Ordem recomendada

```text
Logo/nome
↓
uma frase
↓
GIF/demo
↓
3 linhas de Quick Start
↓
Por que existe
↓
Exemplo real
↓
Como funciona
↓
Performance/benchmark
↓
Privacidade
↓
Integrações
↓
Instalação
↓
Documentação
↓
Roadmap
```

## Quick Start

Deve ser tão curto quanto possível.

Exemplo conceitual:

```text
install
cd project
analyze
search
context
```

Os comandos exatos dependem do nome final do binário.

## O README deve mostrar

- screenshot ou terminal recording;
- exemplo de resultado;
- arquitetura simplificada;
- exemplo de MCP;
- benchmark honesto;
- comparação conceitual com alternativas;
- política de privacidade.

---

# 38. Fase 30 — Documentação técnica

Criar e manter:

```text
README.md
ARCHITECTURE.md
CONTRIBUTING.md
SECURITY.md
CHANGELOG.md
ROADMAP.md
```

E:

```text
docs/
├── getting-started.md
├── configuration.md
├── indexing.md
├── retrieval.md
├── context-format.md
├── mcp.md
├── troubleshooting.md
├── performance.md
├── privacy.md
└── decisions/
```

## Princípio

A documentação deve explicar:

- o que acontece;
- o que não acontece;
- quais dados são usados;
- como o ranking funciona;
- por que um resultado foi selecionado.

---

# 39. Fase 31 — Exemplos e fixtures públicos

## Explicação simples

Uma pessoa precisa conseguir testar a ferramenta sem precisar encontrar um repositório específico.

## Criar exemplos

### Exemplo 1 — TypeScript

Pequeno backend com:

- routes;
- services;
- repository;
- tests.

### Exemplo 2 — Python

API simples com:

- models;
- services;
- routes;
- tests.

### Exemplo 3 — Rust

CLI simples.

## Objetivo

Mostrar:

- map;
- search;
- context;
- MCP.

---

# 40. Fase 32 — Benchmark público de contexto

## Explicação simples

O produto precisa provar que economiza contexto sem destruir relevância.

## Benchmark

Escolher repositórios públicos representativos e registrar:

- número de arquivos;
- tamanho original;
- quantidade de chunks;
- quantidade de símbolos;
- contexto produzido;
- redução estimada;
- perguntas avaliadas;
- resultados esperados;
- resultados obtidos.

## Regra de honestidade

Não usar somente os exemplos em que o produto performa bem.

Documentar:

- limitações;
- linguagens não suportadas;
- casos difíceis;
- falso positivo;
- falso negativo.

---

# 41. Fase 33 — Beta privada / usuários reais

## Explicação simples

Antes da versão 1.0, algumas pessoas devem usar a ferramenta em projetos reais.

## Perfis

Buscar usuários com:

- projetos TypeScript;
- projetos Python;
- projetos Rust;
- repositórios grandes;
- uso frequente de coding agents.

## Perguntas de feedback

- Você entendeu a ferramenta em menos de um minuto?
- O primeiro resultado pareceu correto?
- O resultado ajudou o agente?
- A busca perdeu arquivos importantes?
- O índice ficou pesado?
- O processo pareceu invasivo?
- O MCP foi fácil de integrar?
- O que faria você instalar isso novamente?

## Regra

Feedback deve gerar issues ou decisões documentadas, não uma lista aleatória de features.

---

# 42. Fase 34 — Hardening antes da 1.0

## Checklist funcional

- [ ] CLI estável
- [ ] configuração versionada
- [ ] scanner seguro
- [ ] `.gitignore` respeitado
- [ ] secrets protegidos
- [ ] parser funcionando
- [ ] índice persistente
- [ ] busca lexical
- [ ] context compiler
- [ ] ranking híbrido
- [ ] export Markdown
- [ ] export JSON
- [ ] MCP read-only
- [ ] incremental indexing
- [ ] watch mode opcional
- [ ] Windows funcionando
- [ ] Linux funcionando
- [ ] macOS funcionando
- [ ] releases automatizados
- [ ] documentação completa
- [ ] benchmark
- [ ] security policy

## Critério de estabilidade

O projeto não entra em 1.0 apenas porque “funciona”.

Ele entra em 1.0 quando o núcleo tem:

- contratos claros;
- testes confiáveis;
- comportamento previsível;
- documentação suficiente;
- release reproduzível;
- instalação simples;
- compatibilidade comprovada.

---

# 43. Fase 35 — Versão 1.0

A versão 1.0 deve representar um produto pequeno, completo e confiável, e não uma promessa de todos os recursos futuros.

## Escopo mínimo esperado

```text
scan
parse
index
search
context
map
status
export
MCP
incremental update
```

## Suporte mínimo

- TypeScript;
- JavaScript;
- Python;
- Rust;
- Markdown;
- JSON;
- TOML;
- YAML textual;
- config formats comuns.

## Propriedades

- local-first;
- no API key required;
- no external DB;
- cross-platform;
- single binary;
- read-only MCP;
- deterministic core;
- optional semantic search.

---

# 44. Fase 36 — Pós-1.0: expansão de linguagens

Adicionar linguagens conforme demanda real.

Ordem sugerida:

1. Go;
2. Java;
3. C#;
4. C/C++;
5. Ruby;
6. PHP;
7. outras por contribuição/comunidade.

Cada nova linguagem precisa incluir:

- grammar;
- detector;
- parser adapter;
- symbol extractor;
- fixtures;
- benchmark;
- documentação.

---

# 45. Fase 37 — Grafo de código avançado

Evoluir o grafo de:

```text
file graph
```

para:

```text
symbol graph
```

Com relações como:

- definition;
- reference;
- call;
- inheritance;
- implementation;
- import;
- composition;
- route-to-controller;
- configuration-to-runtime.

A precisão deverá ser mantida como requisito.

---

# 46. Fase 38 — Retrieval orientado a tarefa

Criar perfis:

```text
architecture
feature
bugfix
refactor
debug
onboarding
documentation
```

Cada perfil pode alterar:

- pesos;
- tipos de símbolos;
- prioridade dos testes;
- recência;
- profundidade do grafo;
- formato do contexto.

Isso deve permanecer uma extensão do mesmo retrieval engine.

---

# 47. Fase 39 — Context Memory opcional

Somente depois de o codebase retrieval estar comprovadamente bom.

Possível memória local de:

- decisões técnicas;
- convenções do projeto;
- preferências documentadas;
- problemas conhecidos;
- arquitetura explicada pelo usuário.

Nunca assumir que uma memória inferida é verdade.

Toda memória deve possuir origem e timestamp.

---

# 48. Fase 40 — Histórico Git como contexto opcional

Adicionar sinais como:

- arquivos recentemente alterados;
- commits relacionados;
- autores;
- mudanças em símbolos;
- histórico de um caminho.

Isso deve ser opcional porque usuários podem não usar Git ou podem possuir requisitos de privacidade diferentes.

---

# 49. Fase 41 — Workspace e multi-repo

Depois da versão 1.x, permitir:

```text
workspace
├── frontend
├── backend
├── shared
└── infra
```

O retrieval poderá tratar múltiplos repositórios como uma unidade lógica.

A identidade e os índices de cada repositório continuam independentes.

---

# 50. Fase 42 — Context protocol aberto

Uma evolução importante da visão de longo prazo é criar um formato aberto para representar:

- arquivos;
- símbolos;
- relações;
- chunks;
- scores;
- proveniência;
- contexto selecionado.

O formato deve ser documentado e versionado.

Objetivo:

> permitir que outros projetos consumam o índice/context package sem depender da CLI.

Esse pode se tornar um dos maiores diferenciais técnicos do produto.

---

# 51. Fase 43 — SDK para integrações

Criar SDK/library pública apenas quando as interfaces internas estiverem estáveis.

Possíveis consumidores:

- agentes;
- IDE extensions;
- CI tools;
- documentation generators;
- code review tools;
- local developer assistants.

A SDK deve ser secundária ao CLI.

---

# 52. Fase 44 — Agent infrastructure avançada

Visão futura:

```text
Agent
  │
  ▼
Context API
  │
  ├── code search
  ├── symbol graph
  ├── project map
  ├── conventions
  ├── history
  └── task context
```

O objetivo não é executar o trabalho do agente, mas dar ao agente uma fonte confiável de conhecimento do projeto.

---

# 53. Roadmap resumido por marcos

## Milestone A — Foundation

Fases 0–4.

Resultado:

- repo;
- Rust;
- CLI;
- config;
- scanner.

## Milestone B — Code Intelligence Core

Fases 5–10.

Resultado:

- linguagem;
- parser;
- símbolos;
- grafo;
- chunks;
- SQLite.

## Milestone C — Retrieval Core

Fases 11–16.

Resultado:

- FTS;
- context compiler;
- compressão;
- embeddings;
- hybrid retrieval.

## Milestone D — Agent Integration

Fases 17–20.

Resultado:

- exports;
- Context API;
- MCP;
- integrações.

## Milestone E — Production Quality

Fases 21–28.

Resultado:

- incremental;
- watch;
- benchmark;
- segurança;
- cross-platform;
- releases.

## Milestone F — Public Launch

Fases 29–35.

Resultado:

- README;
- docs;
- examples;
- benchmark;
- beta;
- 1.0.

## Milestone G — Ecosystem

Fases 36–44.

Resultado:

- linguagens;
- graph avançado;
- task retrieval;
- memória;
- Git;
- workspace;
- protocol;
- SDK;
- agent infrastructure.

---

# 54. O que deve existir no final de cada milestone

Cada milestone só fecha quando existir:

```text
Código
↓
Testes
↓
Documentação
↓
Exemplo
↓
Critério mensurável
```

Nunca fechar milestone somente com código compilando.

---

# 55. Estratégia de testes

## Unit tests

Cobrir:

- config;
- ignore rules;
- language detection;
- parsing;
- symbol extraction;
- hashing;
- chunking;
- scoring;
- token estimation;
- serialization.

## Integration tests

Cobrir pipeline:

```text
repo
→ scan
→ parse
→ index
→ search
→ context
```

## E2E tests

Executar o binário real sobre fixtures.

## Snapshot tests

Usar snapshots para:

- CLI output;
- Markdown context;
- JSON schema;
- project map.

Snapshots devem ser revisados conscientemente quando mudarem.

## Regression tests

Toda falha importante encontrada em projeto real deve, quando possível, virar fixture.

---

# 56. Contratos de erro

Os erros internos devem ser tipados.

Categorias conceituais:

```text
ConfigError
ProjectDiscoveryError
ScanError
ParseError
IndexError
SearchError
ContextError
StorageError
McpError
SecurityError
```

Um erro de um arquivo não deve necessariamente derrubar toda a indexação.

Exemplo:

```text
1,000 files
999 parsed
1 parser failure
```

O sistema deve continuar quando for seguro, registrar a falha e reportar o arquivo problemático.

---

# 57. Política de degradação

O produto deve funcionar em níveis.

## Sem Tree-sitter

Arquivo pode ser indexado textualmente quando o parser não existir.

## Sem embeddings

Busca lexical continua funcionando.

## Sem Git

Projeto continua funcionando.

## Sem internet

Core continua funcionando após dependências/modelos necessários já estarem disponíveis.

## Repositório parcialmente quebrado

Parser deve tolerar erros e continuar.

Essa capacidade de degradação é parte essencial do produto.

---

# 58. Política de desempenho

Nunca otimizar com base em sensação.

Sempre medir antes e depois.

As principais métricas serão:

```text
cold index time
warm index time
incremental index time
search latency
context build latency
memory peak
disk footprint
```

Os benchmarks devem ser executados em ambiente documentado.

---

# 59. Política de armazenamento

## Não armazenar por padrão

- credenciais;
- dados de autenticação;
- arquivos binários inúteis;
- conteúdo explicitamente excluído.

## Pode armazenar localmente

- metadata;
- chunks permitidos;
- símbolos;
- relações;
- hashes;
- embeddings locais.

## Limpeza

Adicionar comando futuro para:

```text
cache clear
index reset
```

O usuário deve conseguir remover todos os dados derivados.

---

# 60. Política de logs

Por padrão, logs não devem mostrar conteúdo de arquivos.

Níveis:

```text
error
warn
info
debug
trace
```

`debug` e `trace` precisam ser tratados como potencialmente capazes de exibir informações mais detalhadas e devem continuar evitando secrets.

---

# 61. Política de versionamento

Usar Semantic Versioning após o lançamento público.

Antes da 1.0:

```text
0.x = API ainda evoluindo
```

Depois:

```text
MAJOR = breaking change
MINOR = feature compatível
PATCH = correção compatível
```

O schema de banco e formatos exportados devem possuir suas próprias versões.

---

# 62. Política de compatibilidade

Não prometer compatibilidade eterna antes da 1.0.

Porém:

- config schema deve ser migrável;
- JSON deve ter versão;
- MCP deve seguir o padrão vigente;
- índices antigos devem ser detectados e migrados ou reconstruídos de maneira segura.

---

# 63. Política de dependências

Adicionar uma dependência somente quando ela:

- reduzir complexidade significativamente;
- for mantida de forma saudável;
- possuir licença compatível;
- não introduzir risco desnecessário;
- evitar implementação própria frágil.

Evitar dependências redundantes.

Manter dependências atualizadas de maneira controlada.

Não aceitar upgrade gigante sem rodar testes/benchmarks.

---

# 64. Política de licenciamento

Licença recomendada para o projeto: **MIT**, salvo conflito futuro com dependências/necessidades comerciais.

Todos os componentes de terceiros precisam ser verificados quanto a licença e atribuição.

---

# 65. Política de segurança de terceiros

Antes de incluir:

- grammar;
- parser;
- embedding model;
- library;
- adapter;

verificar:

- licença;
- manutenção;
- dependências transitivas;
- permissões;
- origem;
- integridade quando baixado.

Modelos de embedding devem ser tratados como dependências externas de dados e ter licença documentada.

---

# 66. GitHub como produto

O repositório não é apenas onde o código ficará armazenado.

Ele também será parte da experiência do produto.

## Deve possuir

- nome claro;
- descrição curta;
- topics;
- README forte;
- licença;
- releases;
- changelog;
- security;
- contribution guidelines;
- issue templates;
- pull request template;
- discussions quando houver comunidade suficiente.

## Topics iniciais

Conceitualmente:

```text
ai
codebase
developer-tools
code-search
semantic-search
mcp
coding-agent
llm
context
cli
rust
```

Não colocar topics irrelevantes somente por SEO.

---

# 67. Estratégia de visibilidade

## Princípio

Visibilidade é consequência de produto + distribuição.

Não manipular estrelas.

Não usar scripts de star exchange.

Não comprar stars.

## Conteúdo de lançamento

Produzir demonstrações que mostram:

```text
repository
→ index
→ ask/search
→ relevant files
→ context package
→ agent integration
```

## Conteúdo técnico

Artigos possíveis:

- “Why giving the whole repository to an AI is inefficient”;
- “How a codebase context engine works”;
- “Building a local code intelligence index in Rust”;
- “Hybrid retrieval for coding agents”;
- “MCP as a context interface rather than the context engine itself”.

---

# 68. Estratégia de demonstração

A demo principal deve caber em aproximadamente 30–60 segundos.

Roteiro conceitual:

```text
1. Mostrar repo grande.
2. Rodar analyze/index.
3. Mostrar mapa.
4. Perguntar “where is authentication implemented?”
5. Mostrar resultados relevantes.
6. Gerar contexto limitado a um orçamento.
7. Mostrar agente usando MCP.
8. Mostrar que tudo aconteceu localmente.
```

A demo deve evitar truques artificiais.

---

# 69. Feature matrix para futuro

| Área | V1 | Futuro |
|---|---:|---:|
| File discovery | ✅ | melhorar |
| `.gitignore` | ✅ | melhorar |
| TypeScript | ✅ | melhorar |
| JavaScript | ✅ | melhorar |
| Python | ✅ | melhorar |
| Rust | ✅ | melhorar |
| Go | — | ✅ |
| Java | — | ✅ |
| C# | — | ✅ |
| Symbol extraction | ✅ | avançado |
| Dependency graph | ✅ | avançado |
| SQLite index | ✅ | otimizar |
| FTS5 | ✅ | otimizar |
| Semantic search | opcional | ✅ |
| Hybrid retrieval | ✅ | melhorar |
| Context compiler | ✅ | avançar |
| Markdown export | ✅ | melhorar |
| JSON export | ✅ | schema público |
| MCP | ✅ | avançado |
| Watch | ✅ | melhorar |
| Git history | — | ✅ |
| Memory | — | ✅ |
| Multi-repo | — | ✅ |
| SDK | — | ✅ |
| GUI | — | não prioritário |
| Cloud service | — | não prioritário |
| Agent autonomy | — | não núcleo |

---

# 70. O que deliberadamente NÃO entra no início

Não adicionar antes da necessidade comprovada:

- GUI desktop;
- SaaS;
- conta de usuário;
- dashboard web obrigatório;
- cloud index;
- agente autônomo;
- execução de comandos do projeto;
- edição automática de código;
- sistema de plugins remoto complexo;
- vector database externo;
- dezenas de linguagens;
- modelo proprietário;
- telemetria obrigatória;
- sistema de pagamento.

Isso existe para proteger o projeto contra crescimento descontrolado do escopo.

---

# 71. Ordem exata recomendada para implementação

A sequência operacional consolidada é:

```text
0. Produto / limites
↓
1. Workspace Rust
↓
2. CLI contract
↓
3. Config / discovery
↓
4. Scanner / ignore
↓
5. Security classification
↓
6. Language detection
↓
7. Tree-sitter parsing
↓
8. Graph
↓
9. Chunking
↓
10. SQLite index
↓
11. FTS search
↓
12. Context compiler
↓
13. Context tiers
↓
14. Hybrid retrieval
↓
15. Local embeddings
↓
16. Graph-expanded retrieval
↓
17. Export
↓
18. Internal Context API
↓
19. MCP
↓
20. Agent integrations
↓
21. Incremental indexing
↓
22. Watch
↓
23. Performance
↓
24. Ranking evaluation
↓
25. Security hardening
↓
26. Cross-platform QA
↓
27. Releases
↓
28. Package managers
↓
29. README
↓
30. Docs
↓
31. Examples
↓
32. Public benchmark
↓
33. Beta
↓
34. Hardening
↓
35. v1.0
```

As fases 36–44 são expansão pós-1.0.

---

# 72. Regra para o agente de desenvolvimento

Ao trabalhar neste projeto, o agente de desenvolvimento deve sempre:

1. entender em qual fase está;
2. não implementar fases posteriores sem necessidade;
3. consultar a arquitetura antes de criar módulo novo;
4. preservar as interfaces existentes;
5. escrever testes junto da feature;
6. atualizar documentação quando comportamento público mudar;
7. criar ADR quando tomar decisão estrutural relevante;
8. não esconder limitações;
9. não criar dependência externa apenas para evitar escrever poucas linhas;
10. manter o projeto compilável.

Quando uma fase estiver completa, o agente deve ser capaz de explicar:

- o que foi criado;
- quais testes existem;
- quais limitações permanecem;
- quais critérios de aceitação foram cumpridos;
- qual fase é a próxima.

---

# 73. Definition of Done por feature

Uma feature não está pronta porque “funciona na máquina do desenvolvedor”.

Ela está pronta quando possui:

```text
implementação
+ testes
+ tratamento de erro
+ documentação
+ exemplo quando necessário
+ compatibilidade cross-platform quando aplicável
+ atualização de changelog/ADR quando necessário
```

---

# 74. Definition of Done por fase

Uma fase está pronta quando:

- código compila;
- testes passam;
- regressões críticas foram verificadas;
- documentação está atualizada;
- comportamento público está definido;
- métricas da fase foram avaliadas;
- não existem TODOs críticos escondidos;
- o resultado pode ser demonstrado.

---

# 75. Como decidir se uma feature deve entrar

Usar esta ordem de perguntas:

### Pergunta 1
Resolve o problema central?

### Pergunta 2
É necessária para usuários reais?

### Pergunta 3
Aumenta precisão ou reduz fricção?

### Pergunta 4
Aumenta complexidade desproporcionalmente?

### Pergunta 5
Pode ser implementada como módulo sem contaminar o core?

### Pergunta 6
Existe benchmark ou evidência para justificar?

Se a resposta for “não” para as primeiras três ou “sim” para complexidade desproporcional, adiar.

---

# 76. Indicadores de sucesso técnico

## Primeiro nível

- CLI funciona;
- scanner correto;
- index consistente;
- search correto.

## Segundo nível

- contexto relevante;
- ranking confiável;
- baixa latência;
- incremental indexing eficiente.

## Terceiro nível

- agente consegue consultar via MCP;
- outros projetos conseguem consumir JSON;
- usuários reutilizam a ferramenta em múltiplos projetos.

## Quarto nível

- contribuidores adicionam linguagens;
- terceiros criam integrações;
- projeto ganha forks e adoção.

---

# 77. Indicadores de sucesso no GitHub

Observar:

- stars;
- forks;
- watchers;
- downloads de releases;
- crates.io downloads;
- issues;
- PRs;
- contributors;
- projetos de terceiros usando MCP/SDK;
- menções externas;
- benchmarks reproduzidos por terceiros.

Não considerar stars isoladamente como prova de produto saudável.

---

# 78. Estratégia de lançamento público

Não lançar com uma página vazia.

Antes da publicação principal, preparar:

```text
README
GIF/video
releases
examples
benchmark
security
architecture
installation
MCP guide
```

O repositório precisa parecer utilizável no primeiro acesso.

---

# 79. Política de concorrência

Concorrentes não devem ser tratados como inimigos.

O README pode citar categorias e ferramentas relevantes de forma honesta.

Exemplo de enquadramento:

```text
Existing tools solve parts of this problem:
- repository packing
- semantic code search
- coding agents
- MCP context servers

This project focuses on being the reusable local context layer underneath them.
```

Nunca afirmar superioridade absoluta sem benchmark.

---

# 80. O que pode tornar o projeto realmente especial

A longo prazo, o maior diferencial não deve ser:

> “tem busca semântica”.

Isso é copiável e já é comum.

O diferencial desejado é:

> **um índice de contexto local, estruturado, auditável e reutilizável por diferentes agentes.**

A metáfora é:

```text
Git → fonte do código

Context Engine → fonte de conhecimento estruturado do código

Agent → consumidor desse conhecimento
```

Isso permite que vários agentes usem a mesma camada.

---

# 81. Visão de arquitetura de longo prazo

```text
                    ┌───────────────────────┐
                    │      CODEBASE         │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ DISCOVERY / SCANNER   │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ PARSER / CODE INTEL   │
                    └───────────┬───────────┘
                                │
               ┌────────────────┼────────────────┐
               ▼                ▼                ▼
          Symbols          Relations          Chunks
               │                │                │
               └────────────────┼────────────────┘
                                ▼
                    ┌───────────────────────┐
                    │    LOCAL INDEX        │
                    │ SQLite + FTS + Vec    │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ RETRIEVAL ENGINE      │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ CONTEXT COMPILER      │
                    └───────────┬───────────┘
                                │
               ┌────────────────┼─────────────────┐
               ▼                ▼                 ▼
              CLI           JSON/MD             MCP
               │                │                 │
               └────────────────┼─────────────────┘
                                ▼
                     Coding Agents / Humans
```

---

# 82. Roadmap de maturidade

## Nível 0 — Utility

A ferramenta encontra arquivos.

## Nível 1 — Codebase Search

A ferramenta encontra trechos.

## Nível 2 — Code Intelligence

A ferramenta entende símbolos e relações.

## Nível 3 — Context Compiler

A ferramenta monta contexto dentro de orçamento.

## Nível 4 — Agent Interface

Agentes consultam via MCP/API.

## Nível 5 — Context Infrastructure

Vários agentes usam o mesmo índice local.

## Nível 6 — Universal Context Layer

O índice torna-se uma camada geral de conhecimento do software.

Essa evolução deve ser gradual.

---

# 83. Primeira versão que vale a pena mostrar publicamente

A primeira demo pública convincente não precisa ter tudo.

Ela precisa mostrar:

```text
repo
 ↓
analyze
 ↓
map
 ↓
search
 ↓
context
```

E mostrar claramente:

- arquivos relevantes;
- símbolos;
- linhas;
- relações;
- orçamento;
- output pronto para IA.

O MCP pode ser apresentado em seguida.

---

# 84. Primeira versão que vale a pena lançar oficialmente

A versão pública inicial deve conter:

- Rust binary;
- Windows/Linux/macOS;
- TypeScript/JavaScript/Python/Rust;
- SQLite;
- FTS5;
- context compiler;
- Markdown/JSON;
- segurança básica;
- incremental index;
- MCP read-only;
- documentação;
- releases.

Semantic embeddings podem estar habilitados por padrão somente quando o desempenho e o tamanho do download forem aceitáveis. Caso contrário, devem continuar opcionais.

---

# 85. Checklist final da versão 1.0

## Produto

- [ ] proposta entendível em segundos
- [ ] problema real
- [ ] não é clone superficial de outra ferramenta
- [ ] posicionamento definido

## Código

- [ ] Rust 2024
- [ ] workspace modular
- [ ] erros tipados
- [ ] configuração versionada
- [ ] core independente de provedores de IA

## Indexação

- [ ] scanner
- [ ] gitignore
- [ ] parser
- [ ] symbols
- [ ] relations
- [ ] chunks
- [ ] fingerprints
- [ ] SQLite
- [ ] FTS5

## Retrieval

- [ ] lexical
- [ ] semantic opcional
- [ ] structural
- [ ] ranking híbrido
- [ ] diversity
- [ ] budget
- [ ] provenance

## Agent

- [ ] Context API
- [ ] JSON
- [ ] Markdown
- [ ] MCP read-only

## Segurança

- [ ] secrets
- [ ] logs
- [ ] no telemetry mandatory
- [ ] no remote indexing mandatory
- [ ] no shell execution via MCP

## Cross-platform

- [ ] Windows
- [ ] Linux
- [ ] macOS
- [ ] x64
- [ ] ARM64 roadmap

## Distribuição

- [ ] GitHub Releases
- [ ] checksums
- [ ] crates.io
- [ ] package manager docs

## Comunidade

- [ ] README
- [ ] examples
- [ ] CONTRIBUTING
- [ ] SECURITY
- [ ] CHANGELOG
- [ ] ROADMAP
- [ ] issue templates

---

# 86. Resultado esperado do projeto

Quando o roadmap for concluído em sua parte principal, o usuário deverá conseguir pensar no produto desta maneira:

> “Eu tenho um repositório grande. Não preciso mandar tudo para minha IA. Esta ferramenta já conhece o projeto, encontra as partes relevantes, entende suas relações e entrega apenas o contexto necessário.”

E um agente deverá poder pensar assim:

```text
I need information about X.
        ↓
Query context layer.
        ↓
Retrieve relevant files/symbols.
        ↓
Follow important relationships.
        ↓
Stay within context budget.
        ↓
Receive auditable context.
```

Essa é a essência da arquitetura.

---

# 87. Referências técnicas usadas para decisões de baseline

Estas fontes servem apenas como referência técnica e devem ser rechecadas quando houver mudança relevante de versões.

- Rust Book / Rust 2024 / versão atual documentada: https://doc.rust-lang.org/book/
- Rust Reference: https://doc.rust-lang.org/reference/
- Cargo Book: https://doc.rust-lang.org/cargo/
- Tree-sitter: https://tree-sitter.github.io/tree-sitter/
- Tree-sitter Rust binding: https://github.com/tree-sitter/tree-sitter/tree/master/lib/binding_rust
- SQLite FTS5: https://www.sqlite.org/fts5.html
- fastembed Rust: https://docs.rs/fastembed/latest/fastembed/
- MCP Registry/documentação: https://registry.modelcontextprotocol.io/docs

---

# 88. Referências de mercado e concorrência

A categoria já possui ferramentas que cobrem partes significativas do problema. Alguns exemplos atuais incluem:

- Repomix — compilação de repositórios para contexto de LLM;
- ferramentas de code context com MCP;
- ferramentas de semantic code search;
- coding agents como Aider e Continue.

Esse fato muda a estratégia: o projeto não deve tentar vencer apenas adicionando as mesmas features. Deve buscar uma posição de **infraestrutura de contexto local, interoperável e reutilizável**.

As características específicas dos concorrentes devem ser verificadas novamente antes do lançamento porque esse mercado evolui rapidamente.

---

# 89. Regra final do roadmap

O objetivo não é completar o maior número possível de funcionalidades.

O objetivo é completar a **menor arquitetura capaz de produzir um resultado excepcional**.

A ordem de prioridade é:

```text
Correção
↓
Relevância
↓
Segurança
↓
Performance
↓
UX
↓
Integrações
↓
Escala
↓
Features secundárias
```

Se uma nova feature entrar em conflito com essa ordem, ela deve ser adiada.

A ferramenta só deve crescer quando o núcleo ficar melhor com o crescimento.

---

# 90. Próximo estado esperado após este roadmap

Depois deste arquivo, os documentos técnicos complementares devem detalhar, sem duplicar este roadmap:

```text
01 — CONTEXT PROMPT
02 — ROADMAP
03 — ARCHITECTURE
04 — FUNCTIONAL SPECIFICATION
05 — DATA MODEL / DATABASE SCHEMA
06 — RETRIEVAL AND RANKING SPEC
07 — CONTEXT FORMAT SPECIFICATION
08 — MCP SPECIFICATION
09 — SECURITY / PRIVACY SPEC
10 — TEST PLAN
11 — BENCHMARK PLAN
12 — RELEASE / DISTRIBUTION PLAN
13 — GITHUB / OPEN SOURCE PLAN
```

O roadmap define **quando e por que** cada camada existe.

Os documentos técnicos seguintes devem definir **exatamente como** cada camada deve se comportar.

---

# FIM DO ROADMAP

**Visão final:**

> **AI Codebase Context / Agent Infrastructure** deve evoluir de uma ferramenta local simples de análise de repositórios para uma camada universal de contexto que agentes de software possam consultar de maneira segura, rápida, determinística e interoperável.

> O produto deve ser excelente no núcleo antes de ser grande no ecossistema.
