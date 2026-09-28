# OrganizaMyMac

Utilitário nativo para macOS que mostra o que ocupa espaço no Mac e ajuda a remover o que não é
necessário com segurança: arquivos grandes e antigos, Downloads, duplicados, mapa de espaço e
aplicativos com os restos que deixam.

> Princípio: ser conservador ao remover dados. Tudo vai para a Lixeira, cada item mostra caminho e
> tamanho antes, e pastas do sistema são bloqueadas no código.

## Estado

| Marco | Conteúdo | Situação |
| --- | --- | --- |
| M0 | Base: Tauri + React, comandos Rust, jobs canceláveis com progresso, SQLite, permissões, camada de segurança, logs, fixtures de teste | ✅ |
| M1 | Armazenamento: visão geral dos discos, arquivos grandes e antigos, Downloads, Lixeira, Mostrar no Finder, Visualização Rápida, mover para a Lixeira com revisão | ✅ |

O plano completo está em [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md).

## Funcionalidades

- **Visão geral:** espaço livre e usado de cada disco, atalhos e últimas análises.
- **Scanner:** o que ocupa espaço em uma pasta ou no disco inteiro, com totais lógico e em disco, pastas e maiores arquivos.
- **Grandes e antigos:** filtros por tamanho (100 MB, 500 MB, 1 GB), tempo sem modificação e tipo; arquivos dentro de apps e bibliotecas não são separados.
- **Downloads:** instaladores antigos (pré-selecionados), arquivos grandes, compactados (os já extraídos aparecem como seguros), instaladores recentes, capturas de tela e arquivos antigos.
- **Lixeira:** quanto ocupa e o que tem; esvaziar pede confirmação. Sem Acesso Total ao Disco, dá para pedir ao Finder.
- Em qualquer lista: Visualização Rápida, Mostrar no Finder e Ignorar. Antes de mover, uma revisão mostra cada caminho, o tamanho e o total.

## Stack

- **UI:** Tauri 2, React 19, TypeScript, Vite, Tailwind CSS 4, Zustand, TanStack Query, Lucide
- **Núcleo:** Rust, rayon, BLAKE3, plist, SQLite (`rusqlite`), `trash`
- A UI nunca executa comandos nem acessa o disco: tudo passa por comandos Rust tipados
  ([ARCHITECTURE.md](docs/ARCHITECTURE.md)).

## Segurança e permissões

- [docs/SAFETY.md](docs/SAFETY.md): regras aplicadas antes de cada remoção e os testes de cada uma.
- [docs/PERMISSIONS.md](docs/PERMISSIONS.md): Acesso Total ao Disco (opcional) e como o app evita
  travar em pedidos de permissão.

## Desenvolvimento

Requisitos: macOS 13+, Node 20+, Rust estável, Xcode Command Line Tools.

```bash
npm install
npm run tauri dev
```

Testes do núcleo (unidade e integração com fixtures sintéticas):

```bash
cd src-tauri && cargo test
```

Benchmark de leitura (não altera nada):

```bash
cd src-tauri && cargo run --release --example bench_scan -- ~/
```

Prévia da interface no navegador, com dados fictícios: `npm run dev` e abra http://localhost:1480.

## Privacidade

Sem telemetria. Histórico de análises, registro de operações e estatísticas ficam só no Mac, em
`~/Library/Application Support/dev.galuppo.OrganizaMyMac`.
