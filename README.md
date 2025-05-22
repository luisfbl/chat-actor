# Chat-Actor Webserver

Este é o serviço HTTP principal do monorepo **Chat-Actor**, implementado em Rust com **axum**. Ele expõe rotas REST básicas, serve arquivos estáticos e faz shutdown gracioso.

---

## 🚀 Pré-requisitos

- Rust (versão 1.60+)
- Cargo
- Node.js + pnpm (para build do frontend, caso use `/static`)

---

## 🔧 Instalação & Build

1. Clone o repositório e navegue até o workspace:
   ```bash
   git clone <repo-url>
   cd chat-actor/chat-actor
   ```

2. *Opcional:* gere o frontend estático (se quiser servir via `/static`):
   ```bash
   cd website
   pnpm install
   pnpm build
   cd ../webserver
   ```

3. Instale dependências Rust e compile:
   ```bash
   cd webserver
   cargo fetch
   cargo build --release
   ```

---

## ▶️ Como rodar

Execute o webserver em modo dev ou release:

```bash
# Em modo debug (mais rápido para iteração):
cargo run -p webserver

# Em modo release (otimizado):
cargo run -p webserver --release
```

Por padrão, o servidor escuta em **0.0.0.0:3000**.

---

## 🔍 Endpoints para testar

| Rota                    | Método | Descrição                          |
| ----------------------- | ------ | -----------------------------------|
| `/`                     | GET    | Página HTML de boas-vindas         |
| `/api/hello`            | GET    | JSON: `{ "msg": "Olá do API!"}` |
| `/static/<arquivo>`     | GET    | Arquivos estáticos (gzip + cache)  |

**Exemplos**:

```bash
curl http://localhost:3000/
# <h1>Bem-vindo ao Chat-Actor Webserver!</h1>

curl http://localhost:3000/api/hello
# { "msg": "Olá do API!" }

curl http://localhost:3000/static/index.html
# (conteúdo do seu build frontend)
```

---

## 📦 Funcionalidades chave

- **Rotas REST** com handlers assíncronos
- **ServeDir** + **CompressionLayer** para arquivos estáticos
- **Cache-Control**: `public, max-age=31536000`
- **Graceful shutdown** ao receber `Ctrl+C`

---

## 🎯 Próximos passos

- Adicionar lógica de negócios e outras rotas
- Integrar WebSocket (crate `websocket`)
- Você pode extrair porta ou caminhos para variáveis de ambiente
- Substituir `println!` por `tracing` para logs estruturados

---

Qualquer dúvida ou sugestão, sinta-se à vontade para abrir uma issue ou PR!
