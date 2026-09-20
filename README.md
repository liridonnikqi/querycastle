<img src="static/icon.svg" width="64" alt="QueryCastle">

# QueryCastle

A desktop SQL client for PostgreSQL, Microsoft SQL Server, MySQL, and SQLite.

> This is still an early preview. Expect bugs and missing features. Feedback is welcome!

![Connection list](main-page.png)

![Workbench](workbench.png)

Open a connection, browse the schema, run queries, and edit rows in the grid.

- Saved connections, or paste a connection string
- Schema explorer for tables, views, functions, and sequences
- SQL editor with autocomplete and formatting (`Ctrl+Enter` to run)
- Results grid you can edit in place
- Query tabs, saved queries, and per-connection history
- Uses native OS(MacOS and Windows) keychains to store credentials securely (macOS Keychain and Windows Credential Manager respectively)
- Readonly mode for connections (useful for production databases)
- SSH tunneling for connections

## Run locally

You need Node.js, Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

To build an installer for your machine:

```bash
npm run tauri build
```

## License

MIT. See [LICENSE](LICENSE).
