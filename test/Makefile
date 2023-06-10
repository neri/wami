.PHONY: all clean
.SUFFIXES: .wat .wasm

TARGETS	= fib.wasm global.wasm opr.wasm

.wat.wasm:
	wat2wasm $<

all: $(TARGETS)

clean:
	-rm $(TARGETS)
