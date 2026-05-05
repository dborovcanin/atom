IMAGE_NAME ?= ghcr.io/absmach/atom
IMAGE_TAG ?= latest
DOCKERFILE ?= Dockerfile
BUILD_CONTEXT ?= .

.PHONY: help docker-build build

help:
	@echo "Available targets:"
	@echo "  make docker-build        Build the Atom Docker image"
	@echo "  make build               Alias for docker-build"
	@echo ""
	@echo "Variables:"
	@echo "  IMAGE_NAME=$(IMAGE_NAME)"
	@echo "  IMAGE_TAG=$(IMAGE_TAG)"
	@echo "  DOCKERFILE=$(DOCKERFILE)"
	@echo "  BUILD_CONTEXT=$(BUILD_CONTEXT)"

docker-build:
	docker build \
		-f $(DOCKERFILE) \
		-t $(IMAGE_NAME):$(IMAGE_TAG) \
		$(BUILD_CONTEXT)

build: docker-build
