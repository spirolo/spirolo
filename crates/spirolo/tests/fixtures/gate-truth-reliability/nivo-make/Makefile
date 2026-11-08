pkgs-test: ##@1 packages run tests for all packages
	@echo "${YELLOW}Running test suites for all packages${RESET}"
	@export BABEL_ENV=development; pnpm jest -c ./packages/jest.config.js --workerThreads --rootDir . ./packages/*/tests

