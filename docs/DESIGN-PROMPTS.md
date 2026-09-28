# Design prompts — Liquid Glass

Prompts for generating the icon set and page mockups in Apple's Liquid Glass design language (macOS 26).
Prompts are in English (image and UI generators follow it better); UI copy stays in Brazilian Portuguese.

Each part has a **base block**: paste it before every prompt in that part so the set stays consistent.

The home page follows CleanMyMac's structure as a reference (one Smart Care hero plus five categories), but the
prompts deliberately avoid its branding and exact visuals so the results are original.

| Category | Accent | Groups these modules |
|---|---|---|
| Cuidado inteligente | magenta | Smart Care |
| Limpeza | green | Limpeza avançada, Downloads, Nuvem, Lixeira |
| Segurança | blue | Auditoria de segurança (experimental) |
| Desempenho | orange | Desempenho, barra de menus |
| Aplicativos | teal | Aplicativos, Restos de apps, Atualizações |
| Minha bagunça | purple | Grandes e antigos, Duplicados, Imagens parecidas |
| Mapa de espaço / Scanner | teal / indigo | secondary shortcuts |

"Segurança" is used instead of "Proteção": IMPLEMENTATION.md forbids claiming users are "protected" until there is a
trustworthy security engine.

---

## Part 1 — Icons

Each module icon is a thick glass slab with its own silhouette (the "frame") and a milky-white symbol on top.
The frame gives each module its personality; the symbol says what it does. Frames are original shapes chosen
per module, not copies of any existing icon set.

| Icon | Frame | Tint |
|---|---|---|
| Cuidado inteligente | rounded 8-lobe rosette | magenta → violet |
| Limpeza | rounded droplet / leaf | mint → emerald |
| Segurança | shield | sky blue → deep blue |
| Desempenho | dome (flat bottom, arched top) | amber → burnt orange |
| Aplicativos | horizontal capsule | cyan → teal |
| Minha bagunça | asymmetric pebble | lavender → purple |
| Mapa de espaço | rounded diamond (square at 45°) | aqua → green-teal |
| Scanner | thick lens ring | periwinkle → indigo |
| App icon | macOS squircle (required by macOS 26) | blue → violet |

### Base block

```
Original 3D icon in Apple's "Liquid Glass" style (macOS 26). A thick slab of frosted,
translucent tinted glass with a distinctive custom silhouette (the "frame"), seen from
the front with a slight top-down three-quarter tilt so its thick side wall is visible
along the bottom edge. The slab has a softly raised rim and rounded bevel all around,
a glossy highlight along the edges, milky internal diffusion, and a vertical color
gradient: lighter and more transparent at the top, deeper and more saturated at the
bottom, with subtle internal caustics. On top of the slab sits one simple symbol made of
soft, matte, milky-white frosted glass, slightly tinted by the color beneath, with rounded
edges, a small thickness and a soft contact shadow. Studio lighting from above, soft
reflections, pure white background, only a faint shadow. Centered, 1024×1024. No text,
no letters, no logos, no brand marks, not resembling any existing app's icons.
```

### 1. Cuidado inteligente (Smart Care)

```
Frame: a rounded rosette with eight soft, shallow lobes, like a gently scalloped badge.
Tint: magenta at the top melting into deep violet at the bottom.
Symbol: three sparkles, one large four-point star and two small ones at its upper right.
```

### 2. Limpeza

```
Frame: a rounded droplet / leaf shape with its point toward the upper right, very soft
curves, thick glass.
Tint: fresh mint at the top to rich emerald at the bottom.
Symbol: a chunky eraser tilted 30°, with three tiny white glass dots trailing behind it.
```

### 3. Segurança

```
Frame: a classic shield with a softly rounded top edge and a rounded bottom point.
Tint: light sky blue at the top to deep sapphire blue at the bottom.
Symbol: a bold, rounded checkmark. Calm and trustworthy, no lock, no warning signs.
```

### 4. Desempenho

```
Frame: a dome: flat bottom edge with rounded corners and a wide semicircular arched top.
Tint: warm amber at the top to burnt orange at the bottom.
Symbol: a speedometer needle pointing to the upper right from a round pivot, with three
short tick marks following the arch of the frame.
```

### 5. Aplicativos

```
Frame: a wide horizontal capsule (stadium shape), thick and soft.
Tint: bright cyan at the top to deep teal at the bottom.
Symbol: a 2×2 grid of small rounded squares; the top-right square is slightly raised.
```

### 6. Minha bagunça

```
Frame: an asymmetric, organic pebble shape, slightly wider at the bottom left, like a
smooth river stone.
Tint: soft lavender at the top to rich purple at the bottom.
Symbol: two document sheets fanned slightly, the front one with a folded corner.
```

### 7. Mapa de espaço

```
Frame: a rounded diamond (a square rotated 45° with generously rounded corners).
Tint: aqua at the top to green-teal at the bottom.
Symbol: an upright small square divided into unequal rectangles (one large, two medium,
three small) separated by thin gaps, like a treemap.
```

### 8. Scanner

```
Frame: a thick circular lens ring: a round slab with a raised outer rim and a slightly
recessed, clearer center, like a magnifying lens seen from the front.
Tint: periwinkle at the top to deep indigo at the bottom.
Symbol: a small folder in the recessed center, slightly magnified and distorted by the
lens around it.
```

### 9. App icon (OrganizeMyMac)

macOS 26 masks app icons to the squircle, so the app icon keeps that frame; personality comes from the symbol.

```
Frame: the standard macOS app icon squircle (rounded square), same thick glass slab.
Tint: electric blue at the top to violet at the bottom, with a faint turquoise glow in
the center.
Symbol: a chunky, soft computer monitor on a short stand; on its screen, three small
rounded blocks arranged as a neat staircase, the top one catching a bright highlight.
The symbol is the hero: large, filling about 60% of the frame.
```

### Template variant (menu bar and sidebar)

Append to any icon prompt above:

```
Monochrome template version: the symbol only, no glass frame, single-color black glyph
on a transparent background, uniform stroke weight, no gradient, no glass effect.
```

---

## Part 2 — Pages

### Base block

```
High-fidelity UI mockup of a native macOS 26 desktop app in Apple's "Liquid Glass"
design language. Window 1280×800 with traffic-light buttons at top left. A left sidebar
in frosted translucent glass shows the wallpaper softly blurred behind it; the main
content area is a calm, slightly lighter surface. Controls, cards and toolbars are
floating glass panels with rounded corners (16–20 px), subtle specular edges and soft
shadows. Typography: SF Pro, clear hierarchy, generous spacing. Each module has its own
accent tint (Smart Care magenta, Limpeza green, Segurança blue, Desempenho orange,
Aplicativos teal, Minha bagunça purple). All UI text in Brazilian Portuguese.
Realistic but fictional data. Tone: calm, honest, never alarmist — no red "danger"
banners, no fake urgency. Every destructive action says "Mover para a Lixeira", never
"Apagar". Light mode unless stated.

Sidebar items (with glass icons): Início, Cuidado inteligente, Limpeza, Segurança,
Desempenho, Aplicativos, Minha bagunça, then a divider, Mapa de espaço, Scanner,
and Ajustes at the bottom.
```

### 1. Início

```
Selected sidebar item: Início.
Top: title "Início" and subtitle "Você já liberou 38,4 GB com o OrganizeMyMac."
Row of two glass disk cards: "Macintosh HD — 182 GB livres de 494 GB" with a usage
bar at 63%, and "Backup (externo) — 1,2 TB livres de 2 TB".
Center hero: a large circular glass button with the Smart Care sparkle icon, magenta
glow, label "Analisar" and caption "Uma análise completa com recomendações para revisar".
Below: five equal glass category cards in a row, each with its large glass icon, name
and one-line status: Limpeza "12,6 GB para revisar", Segurança "Nenhum item suspeito",
Desempenho "Memória 71% em uso", Aplicativos "3 atualizações disponíveis",
Minha bagunça "214 duplicados encontrados".
Bottom: two small shortcut chips "Mapa de espaço" and "Scanner", then a compact list
"Últimas análises" with 4 rows (date, module, path, size).
```

### 2. Cuidado inteligente

```
Selected: Cuidado inteligente. Magenta accent.
State: results after an analysis. Top: large number "18,2 GB" with caption
"recomendados para revisão" and a secondary button "Analisar de novo".
Grid of recommendation cards, each with a checkbox, icon, title, size and a
"Revisar" link: "Caches de sistema 4,1 GB" (checked), "Instaladores antigos em
Downloads 6,3 GB" (checked), "Restos de apps removidos 1,8 GB" (checked),
"Duplicados 2,4 GB" (unchecked), "Lixeira 3,6 GB" (unchecked, with note
"Nunca vem marcada").
Sticky glass footer: "3 itens selecionados · 12,2 GB" and a primary button
"Revisar e mover para a Lixeira".
```

### 3. Limpeza

```
Selected: Limpeza. Green accent.
Segmented glass control at top: "Limpeza avançada · Downloads · Nuvem · Lixeira",
with "Limpeza avançada" active.
Left column: list of rule groups with sizes and checkboxes: "Caches de usuário 3,2 GB",
"Logs 640 MB", "Caches de Xcode 5,8 GB", "Caches de navegadores 1,1 GB".
One row is dimmed with a small lock badge: "Caches do Safari — app aberto, bloqueado".
Right column: detail panel for the selected rule listing file paths, sizes, last
modified dates, and a small "Regra embutida" tag.
Footer: "4 regras · 10,7 GB" and button "Revisar e mover para a Lixeira".
```

### 4. Segurança

```
Selected: Segurança. Blue accent. Small pill badge "Experimental" next to the title.
Honest subtitle: "Verifica itens de inicialização e assinaturas de apps. Não substitui
um antivírus."
Two sections as glass tables:
"Itens de inicialização" — rows for LaunchAgents/LaunchDaemons with name, developer,
path, and a status chip (Assinado pela Apple / Developer ID / Sem assinatura).
"Assinaturas de apps" — rows for apps with signature status; one row shows a neutral
yellow chip "Sem assinatura — revisar".
Summary card at top: "64 apps verificados · 0 problemas graves · 1 para revisar".
No scary imagery.
```

### 5. Desempenho

```
Selected: Desempenho. Orange accent.
Top row of four glass stat tiles with sparklines: CPU "23%", Memória "11,4 de 16 GB",
Bateria "82% · 4 h 10 min", Disco "182 GB livres".
Below: table "Apps que mais consomem" with columns App, CPU, Memória, Energia,
and an "Encerrar" button on each row (secondary style).
Right side: small card "Barra de menus" with a toggle "Mostrar no menu" and a
preview of the menu bar dropdown.
```

### 6. Aplicativos

```
Selected: Aplicativos. Teal accent.
Segmented control: "Aplicativos · Restos de apps · Atualizações", first active.
Grid/list of installed apps with icon, name, version, size, last used date.
One app is selected and a glass side sheet shows the uninstall preview grouped by
confidence: "Seguro" (green chips: bundle, Application Support, Caches),
"Revisar" (yellow chips: files matched by developer name),
"Risco" (neutral-gray chip: Group Container shared with other apps, unchecked).
Button at bottom of the sheet: "Desinstalar e mover para a Lixeira — 2,3 GB".
```

### 7. Minha bagunça

```
Selected: Minha bagunça. Purple accent.
Segmented control: "Grandes e antigos · Duplicados · Imagens parecidas",
"Duplicados" active.
List of duplicate groups, each a glass card showing 2–4 copies of the same file
with path, date and size; one copy is marked with a star "Manter" and cannot be
unchecked; a caption "Uma cópia sempre é mantida".
Top summary: "214 grupos · 2,4 GB recuperáveis".
Toolbar: sensitivity for images "Estrito / Normal" (disabled here), filter by type.
Footer button "Mover cópias selecionadas para a Lixeira".
```

### 8. Mapa de espaço

```
Selected: Mapa de espaço. Teal accent.
Breadcrumb at top: "Macintosh HD › Usuários › augusto › Library".
The main area is a large squarified treemap made of translucent glass tiles, sized by
folder size, tinted by file type (apps, media, documents, caches, other), with folder
names and sizes inside the larger tiles. Hovering one tile shows a glass tooltip
"Developer · 42,1 GB · 318 mil itens".
Right mini panel: legend by type and the 5 largest folders.
```

### 9. Scanner

```
Selected: Scanner. Indigo accent.
State: scanning in progress. Top: path picker "Pasta: ~/" and button "Cancelar".
Progress glass card: animated indeterminate bar, "1,2 milhão de itens · 214 GB lidos ·
9 s", and a soft note "Algumas pastas precisam de Acesso Total ao Disco".
Below: live-updating table of the largest folders found so far (name, size, items).
```

### 10. Ajustes

```
Selected: Ajustes (bottom of sidebar).
Grouped glass sections like macOS System Settings:
"Geral" — toggle "Mostrar o OrganizeMyMac na barra de menus".
"Permissões" — status row "Acesso Total ao Disco: concedido" with a button
"Abrir Ajustes do Sistema"; "Fotos: não solicitado".
"Pastas ignoradas" — list of 2 folders with remove buttons and "Adicionar pasta…".
"Sobre" — "OrganizeMyMac 0.1.0", data and log folder paths.
```

---

## Tips

- Icons: generate Limpeza and Segurança first, pick the best result, and pass it as a style
  reference image for the rest so the set stays coherent.
- Pages: with UI generators (Figma Make, v0, Stitch), attach a screenshot of the current page alongside the prompt
  to keep the data structure.
