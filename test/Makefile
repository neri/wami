.PHONY: all clean
.SUFFIXES: .wat .wasm

TARGETS	= tester.wasm

.wat.wasm:
	wat2wasm $<

all: $(TARGETS)

clean:
	-rm $(TARGETS)
