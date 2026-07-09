# ─── Harsh Realm — Frontend (development) ────────────────────────────────────
# Runs the Vite dev server with hot-module-replacement.
# Mount the host src/ directory for live editing.
#
# docker build -f frontend/Dockerfile -t harsh-realm-frontend ./frontend
# ──────────────────────────────────────────────────────────────────────────────
FROM node:24-alpine

WORKDIR /app

# Install dependencies in a separate layer (cached unless package.json changes)
COPY package.json package-lock.json* ./
RUN npm install

# Source is bind-mounted at runtime; this copy just ensures the image
# works standalone if needed (e.g. in CI).
COPY . .

# Vite must bind to 0.0.0.0 so the mapped port is reachable from the host.
# VITE_BACKEND_URL and VITE_WS_BACKEND_URL override the proxy targets
# (set in docker-compose.yml to point at the backend service).
EXPOSE 5173

CMD ["npm", "run", "dev"]
