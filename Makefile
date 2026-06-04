IMAGE_NAME ?= ghcr.io/absmach/atom
IMAGE_TAG ?= latest
BUILD_TARGET ?= release
DOCKERFILE ?= Dockerfile
BUILD_CONTEXT ?= .
COMPOSE ?= docker compose
COMPOSE_PROFILES ?= --profile default --profile atom-ui
DEV_ENV_FILE ?= .env.dev

.PHONY: help dev-env build atom-build ui-build up down logs restart docker-build docker-build-release docker-build-dev

help:
	@echo "Available targets:"
	@echo "  make build               Build Atom backend + Atom UI images for local dev"
	@echo "  make atom-build          Build only the Atom backend Compose image"
	@echo "  make ui-build            Build only the Atom UI Compose image"
	@echo "  make up                  Build and start Postgres, Atom, and Atom UI"
	@echo "  make restart             Rebuild and restart Postgres, Atom, and Atom UI"
	@echo "  make logs                Follow Atom + Atom UI logs"
	@echo "  make down                Stop the local Compose stack"
	@echo "  make dev-env             Create $(DEV_ENV_FILE) with local dev defaults"
	@echo "  make docker-build        Build the raw Atom Docker image for BUILD_TARGET"
	@echo "  make docker-build-release Build the raw release Docker image"
	@echo "  make docker-build-dev    Build the raw dev Docker image"
	@echo ""
	@echo "Variables:"
	@echo "  COMPOSE=$(COMPOSE)"
	@echo "  COMPOSE_PROFILES=$(COMPOSE_PROFILES)"
	@echo "  DEV_ENV_FILE=$(DEV_ENV_FILE)"
	@echo "  IMAGE_NAME=$(IMAGE_NAME)"
	@echo "  IMAGE_TAG=$(IMAGE_TAG)"
	@echo "  BUILD_TARGET=$(BUILD_TARGET)"
	@echo "  DOCKERFILE=$(DOCKERFILE)"
	@echo "  BUILD_CONTEXT=$(BUILD_CONTEXT)"

dev-env:
	@if [ -f "$(DEV_ENV_FILE)" ]; then \
		echo "$(DEV_ENV_FILE) already exists"; \
	else \
		{ \
			echo "POSTGRES_USER=atom"; \
			echo "POSTGRES_PASSWORD=atom"; \
			echo "POSTGRES_DB=atom"; \
			echo "ADMIN_SECRET=12345678"; \
			echo "ATOM_MIN_PASSWORD_CHARS=8"; \
			echo "ATOM_SERVICE_SECRET=atom-dev-service-password"; \
			echo "ATOM_CERTS_KEY_ENCRYPTION_SECRET=atom-dev-certs-key-encryption-secret"; \
			echo "ATOM_SIGNUP_ENABLED=false"; \
			echo "ATOM_HTTP_PORT=18080"; \
			echo "ATOM_PUBLIC_BASE_URL=http://localhost:18080"; \
			echo "ATOM_UI_HTTP_PORT=3005"; \
			echo "ATOM_GRAPHQL_URL=http://atom:8080/graphql"; \
		} > "$(DEV_ENV_FILE)"; \
		echo "Created $(DEV_ENV_FILE) with local developer defaults"; \
	fi

build: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) build atom atom-ui

atom-build: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) build atom

ui-build: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) build atom-ui

up: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) up -d --build postgres atom atom-ui

restart: down up

logs: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) logs -f atom atom-ui

down: dev-env
	$(COMPOSE) --env-file $(DEV_ENV_FILE) $(COMPOSE_PROFILES) down

docker-build:
	docker build \
		-f $(DOCKERFILE) \
		--target $(BUILD_TARGET) \
		-t $(IMAGE_NAME):$(IMAGE_TAG) \
		$(BUILD_CONTEXT)

docker-build-release:
	$(MAKE) docker-build BUILD_TARGET=release IMAGE_TAG=release

docker-build-dev:
	$(MAKE) docker-build BUILD_TARGET=dev IMAGE_TAG=dev
