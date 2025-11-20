.PHONY: all libmemscan pymemscan climemscan clean

all: libmemscan pymemscan climemscan

libmemscan:
	cargo build --release -p libmemscan

pymemscan:
	$(MAKE) -C pymemscan build

climemscan:
	cargo build --release -p climemscan

clean:
	cargo clean -p libmemscan
	$(MAKE) -C pymemscan clean
	cargo clean -p climemscan
