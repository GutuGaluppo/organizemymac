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

### Base block

```
Original macOS app icon in Apple's "Liquid Glass" design language (macOS 26).
A single bold symbol made of thick, translucent glass floating above a softly tinted
glass rounded-square (squircle) base. The glass shows real refraction and light bending
at the edges, a crisp specular highlight along the top-left rim, subtle inner glow,
soft frosted translucency, and a gentle drop shadow that gives depth. Colors come from
light passing through tinted glass, not from flat paint. Minimal, geometric, no
fine detail, readable at 16 px. Centered, front-facing, slight top-down lighting.
Neutral light-gray background, 1024×1024, no text, no letters, no logos, no brand marks.
```

### 1. Cuidado inteligente (Smart Care)

```
Symbol: three sparkles (one large four-point star, two small ones) in glass.
Tint: magenta-to-violet gradient glass. The large sparkle has a bright caustic
highlight at its center, as if catching light. Feeling: effortless, one-click care.
```

### 2. Limpeza

```
Symbol: an eraser tilted 30°, with a short trail of three tiny glass particles
fading behind it. Tint: fresh green-to-mint glass. Clean, satisfying, calm.
```

### 3. Segurança

```
Symbol: a shield with a checkmark cut through it, the check showing the base color
beneath through the glass. Tint: deep blue-to-cyan glass. Sober and trustworthy,
not aggressive, no lock, no padlock, no warning signs.
```

### 4. Desempenho

```
Symbol: a semicircular speedometer gauge with a needle pointing to the upper right,
three glass tick marks on the arc. Tint: warm orange-to-amber glass, with a slight
inner glow at the needle pivot. Energetic but controlled.
```

### 5. Aplicativos

```
Symbol: a 2×2 grid of rounded-square glass tiles, one tile slightly lifted and
offset toward the viewer. Tint: sky-blue-to-teal glass with each tile a slightly
different shade. Organized, modular.
```

### 6. Minha bagunça

```
Symbol: two overlapping document sheets fanned slightly, the front one with a
folded corner, the back one showing through the translucent front sheet.
Tint: purple-to-lavender glass. Personal files, gentle, non-judgmental.
```

### 7. Mapa de espaço

```
Symbol: a treemap: a square split into rectangles of unequal sizes (one large,
two medium, three small), separated by thin clear glass gaps, each rectangle a
slightly different tint. Tint family: teal-to-green glass. Analytical, spatial.
```

### 8. Scanner

```
Symbol: a folder with a magnifying glass in front of it, the lens actually magnifying
and distorting the folder behind it through refraction. Tint: indigo-to-blue glass.
Curious, investigative.
```

### 9. App icon (OrganizeMyMac)

```
Symbol: an open glass box with three small rounded glass blocks neatly stacked inside
and one block hovering above, about to be placed. Tint: blue-to-violet glass with a
bright specular highlight on the hovering block. Conveys "organizing your Mac": tidy,
safe, friendly. Premium Apple-quality craftsmanship.
```

### Template variant (menu bar and sidebar)

Append to any icon prompt above:

```
Monochrome template version: same symbol only, no base squircle, single-color
black glyph on transparent background, uniform 2 px-equivalent stroke weight,
no gradient, no glass effect.
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

- Icons: generate the app icon plus Limpeza and Segurança first, pick the best result, and pass it as a style
  reference image for the rest so the set stays coherent.
- Pages: with UI generators (Figma Make, v0, Stitch), attach a screenshot of the current page alongside the prompt
  to keep the data structure.
