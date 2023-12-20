.PHONY: all clean
.SUFFIXES: .wat .wasm

TARGETS	= tester.wasm

.wat.wasm:
	wat2wasm $< --debug-names

all: $(TARGETS)

clean:
	-rm $(TARGETS)
