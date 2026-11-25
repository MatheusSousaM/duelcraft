# DuelCraft

Aplicativo desktop leve (**Rust + Tauri v2 + React + TypeScript**) para montagem, análise e otimização de decks de *Yu-Gi-Oh! Master Duel*.

## Recursos

- **Editor de Deck estilo Master Duel:** busca local instantânea nas ~14.000 cartas com filtros por tipo, subtipo (Fusão, Sincro, Xyz, Link, Ritual, Pêndulo), atributo, nível, Link Rating, raça, raridade e cartas relacionadas, seguindo a ordenação oficial do jogo.
- **Otimizador de Deck & Planejamento de CP:** compara sua lista com decks competitivos do mesmo arquétipo (sem viés contra decks Rogue), sugere cortes/adições com justificativa ("Antes vs Depois") e divide o custo de CP em etapas + recomendações de *Secret Packs*.
- **Meta Decks & Tier List:** consulta decks recentes de *Master I*, *Master V*, *Duelist Cup* e *Campeonatos*, com expansão automática por arquétipo.
- **Importação e Exportação `.YDK`:** compatível com EDOPro, YGO Omega, DuelingBook e backups locais, além de montagem assistida na janela do jogo.

## Requisitos para Desenvolvimento

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable) + Build Tools C++ do Visual Studio (Windows)

```bash
npm install
npm run tauri dev
```

Para gerar o executável otimizado de produção:

```bash
npx tauri build --no-bundle
```

## Aviso Legal

- **Projeto de fã independente**, sem qualquer vínculo, patrocínio ou afiliação com a **Konami Digital Entertainment** ou com o **Master Duel Meta**.
- Nomes, atributos e artes das cartas de *Yu-Gi-Oh!* são marcas e propriedades registradas da Konami. Dados públicos de torneios e catálogo consultados via Master Duel Meta.
- O recurso opcional de preenchimento automático ("Montar no Jogo") simula entradas de mouse e teclado na janela local do editor de deck; utilize por sua própria conta e risco conforme os Termos de Uso do jogo.
