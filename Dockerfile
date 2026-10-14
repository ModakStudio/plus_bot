# ==========================================
# Stage 1: Builder (컴파일 환경)
# ==========================================
FROM rust:slim-bookworm AS builder

# 네트워크 통신(OpenSSL) 빌드에 필요한 시스템 패키지 설치
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# 1. 의존성 캐싱 단계: Cargo.toml과 Cargo.lock만 먼저 복사
COPY Cargo.toml Cargo.lock ./

# 가짜(Dummy) main.rs를 만들어 의존성 패키지들만 먼저 컴파일 (레이어 캐싱)
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 2. 실제 소스 코드 복사 및 본 빌드
COPY src ./src
# 가짜 바이너리 흔적을 지워 변경된 소스가 확실히 다시 컴파일되도록 타임스탬프 갱신
RUN touch src/main.rs && cargo build --release

# ==========================================
# Stage 2: Runtime (실제 구동 환경)
# ==========================================
FROM debian:bookworm-slim

# 중요: 디스코드 및 노션 HTTPS API 통신을 위해 루트 인증서(ca-certificates)가 필수입니다!
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Builder 스테이지에서 완성된 바이너리 파일(plus_bot)만 쏙 가져옴
COPY --from=builder /usr/src/app/target/release/plus_bot /app/plus_bot

# 봇 실행
CMD ["./plus_bot"]