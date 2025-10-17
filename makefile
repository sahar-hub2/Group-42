# GROUP: 42
# MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

.PHONY: start stop

# Start server in Docker and run client locally
start:
	docker-compose up -d server
	@echo "Waiting for server to be ready..."
	@while ! nc -z localhost 3000; do sleep 1; done
	cd client && bun install && bun run tauri dev

# Stop the Docker server
stop:
	docker-compose down
