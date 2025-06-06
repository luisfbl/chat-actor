FROM node:18-alpine as builder

WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Copy root package files for workspace
COPY pnpm-workspace.yaml pnpm-lock.yaml ./
COPY website/package.json ./website/

# Install dependencies for workspace
RUN pnpm install --frozen-lockfile

# Copy website source code
COPY website/ ./website/

# Build the website
RUN pnpm --filter website build

# Production stage
FROM nginx:alpine

# Copy build files to nginx (website builds to website/dist)
COPY --from=builder /app/website/dist /usr/share/nginx/html

# Copy nginx configuration if needed
# COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]