# syntax=docker/dockerfile:1
# Build from project root: docker build -f web-next/Dockerfile -t bitbridge-web-next:latest .

FROM --platform=linux/amd64 midwess/deno:builder2.5.2 AS builder
WORKDIR /app
COPY . .

ARG BYTOVER_PUBLIC_GATEWAY_HOST
ARG BYTOVER_PUBLIC_GATEWAY_PORT
ARG BYTOVER_WITH_SSL
ARG BYTOVER_LOCATOR_URL
ARG AWS_ACCESS_KEY_ID
ARG AWS_SECRET_ACCESS_KEY
ARG AWS_REGION
ARG AWS_S3_BUCKET_NAME
ARG S3_CDN_PREFIX
ARG VERSION
ARG RAILWAY_GIT_COMMIT_SHA

ENV BYTOVER_PUBLIC_GATEWAY_HOST=${BYTOVER_PUBLIC_GATEWAY_HOST}
ENV BYTOVER_PUBLIC_GATEWAY_PORT=${BYTOVER_PUBLIC_GATEWAY_PORT}
ENV BYTOVER_WITH_SSL=${BYTOVER_WITH_SSL}
ENV BYTOVER_LOCATOR_URL=${BYTOVER_LOCATOR_URL}
ENV AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
ENV AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
ENV AWS_REGION=${AWS_REGION}
ENV AWS_S3_BUCKET_NAME=${AWS_S3_BUCKET_NAME}
ENV S3_CDN_PREFIX=${S3_CDN_PREFIX}
ENV VERSION=${VERSION}
ENV RAILWAY_GIT_COMMIT_SHA=${RAILWAY_GIT_COMMIT_SHA}


WORKDIR /app/web-next

ENV NODE_ENV=production

RUN cd /app/shared_types && cargo build --features typescript

RUN pnpm wasm:build

RUN pnpm install --frozen-lockfile

RUN pnpm next build

FROM --platform=linux/amd64 midwess/deno:runner2.5.2 AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000

# Copy package.json and lock file
COPY --from=builder /app/web-next/package.json ./
COPY --from=builder /app/web-next/pnpm-lock.yaml ./

# Copy built application
COPY --from=builder /app/web-next/public ./public
COPY --from=builder /app/web-next/.next/standalone ./
COPY --from=builder /app/web-next/.next/static ./.next/static

# Copy node_modules (needed for pnpm start)
COPY --from=builder /app/web-next/node_modules ./node_modules

EXPOSE 3000

CMD ["deno", "task", "deno-next", "start", "--turbopack"]
