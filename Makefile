EXE = monty

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	OLD := monty-$(VER).exe
	AVX2 := monty-$(VER)-avx2.exe
else
	NAME := $(EXE)
	OLD := monty-$(VER)
	AVX2 := monty-$(VER)-avx2
endif

rule:
	cargo rustc --release --bin monty -- -C target-cpu=native --emit link=$(NAME)

ataxx:
	cargo rustc --release --bin monty --features ataxx -- -C target-cpu=native --emit link=$(NAME)

shatranj:
	cargo rustc --release --bin monty --features shatranj -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release --bin monty -- --emit link=$(OLD)
	cargo rustc --release --bin monty -- -C target-cpu=x86-64-v2 -C target-feature=+avx2 --emit link=$(AVX2)