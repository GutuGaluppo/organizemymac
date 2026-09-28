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
| M2 | Duplicados: grupos por tamanho, hash de trechos, BLAKE3 completo, seleção automática que sempre mantém uma cópia, hard links reconhecidos | ✅ |
| M3 | Mapa de espaço: treemap proporcional (squarified), lista hierárquica, navegação por pastas, trilha de navegação e seleção | ✅ |
| M4 | Aplicativos: lista com tamanho, versão e último uso; desinstalação com prévia completa dos arquivos em ~/Library por nível de confiança; restos de apps desinstalados | ✅ |
| M5 | Saúde do Mac e barra de menus: CPU, memória, swap, disco, bateria, tempo ligado, apps abertos e os que mais usam memória | ✅ |

M0–M4 formam o MVP 1.0 recomendado no plano. O plano completo está em [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md).

## Funcionalidades

- **Visão geral:** espaço livre e usado de cada disco, atalhos e últimas análises.
- **Scanner:** o que ocupa espaço em uma pasta ou no disco inteiro, com totais lógico e em disco, pastas e maiores arquivos.
- **Mapa de espaço:** retângulos com área proporcional ao espaço, dois níveis por vez; clique numa pasta para entrar, trilha de navegação para voltar, lista hierárquica ao lado e seleção de pastas ou arquivos para a Lixeira. Usa a análise do Scanner, sem ler o disco de novo.
- **Grandes e antigos:** filtros por tamanho (100 MB, 500 MB, 1 GB), tempo sem modificação e tipo; arquivos dentro de apps e bibliotecas não são separados.
- **Downloads:** instaladores antigos (pré-selecionados), arquivos grandes, compactados (os já extraídos aparecem como seguros), instaladores recentes, capturas de tela e arquivos antigos.
- **Duplicados:** agrupa por tamanho, compara trechos do início, meio e fim e só então calcula o BLAKE3 completo (em ~/Projects: 8.307 candidatos, 1.616 lidos por inteiro, 1,8 GB/s). A seleção automática mantém a cópia fora de Downloads e da Lixeira, sem "cópia" no nome e mais antiga; o app recusa remover todas as cópias de um grupo e pula as que mudaram desde a análise.
- **Lixeira:** quanto ocupa e o que tem; esvaziar pede confirmação. Sem Acesso Total ao Disco, dá para pedir ao Finder.
- **Aplicativos:** apps de /Applications e ~/Applications com tamanho, versão e último uso (Spotlight); filtros "sem uso há 6 meses" e App Store. Apps do macOS são protegidos; apps abertos precisam ser encerrados antes.
- **Desinstalar:** o app e os arquivos dele em ~/Library, com o motivo de cada item e o total por local. Identificador exato do app e pastas conhecidas (rules/apps) são *seguros* e já vêm marcados; pasta do fabricante ou com nome parecido pede *revisão*; contêiner compartilhado é *risco*. O servidor refaz o plano e só aceita caminhos que fazem parte dele.
- **Restos de apps:** arquivos em ~/Library com o identificador de um app que não está em nenhum disco indexado pelo Spotlight. Identificadores da Apple são ignorados; se o fabricante ainda tem apps instalados, o item fica para revisão.
- **Desempenho:** CPU com histórico, memória e swap, disco, bateria (pmset), tempo ligado e processos separados em aplicativos (helpers somados ao app), segundo plano e sistema. "Encerrar" só aparece para apps comuns seus e equivale a escolher Encerrar no menu do app (nunca `kill -9`). Atualiza a cada 2 s e para quando a janela está escondida.
- **Barra de menus:** CPU e/ou memória no título; no menu, disco, bateria, tempo ligado e os 5 apps que mais usam memória. Métricas a cada 5 s e processos a cada 30 s, em prioridade baixa (≈0,4% de um núcleo no build de desenvolvimento). Com ela ligada, fechar a janela mantém o app na barra.
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

Testes do núcleo (unidade e integração com fixtures sintéticas) e da interface:

```bash
cd src-tauri && cargo test
```

```bash
npm test
```

Benchmark de leitura (não altera nada):

```bash
cd src-tauri && cargo run --release --example bench_scan -- ~/
```

Prévia da interface no navegador, com dados fictícios: `npm run dev` e abra http://localhost:1480.

## Privacidade

Sem telemetria. Histórico de análises, registro de operações e estatísticas ficam só no Mac, em
`~/Library/Application Support/dev.galuppo.OrganizaMyMac`.
