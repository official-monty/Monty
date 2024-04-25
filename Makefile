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

chess:
	cargo rustc --release --package monty --bin monty -- -C target-cpu=native --emit link=$(NAME)

ataxx:
	cargo rustc --release --package monty --bin montyxx -- -C target-cpu=native --emit link=$(NAME)

shatranj:
	cargo rustc --release --package monty --bin montyj -- -C target-cpu=native --emit link=$(NAME)

gen:
	cargo rustc --release --package datagen --bin monty -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release --bin monty -- --emit link=$(OLD)
	cargo rustc --release --bin monty -- -C target-cpu=x86-64-v2 -C target-feature=+avx2 --emit link=$(AVX2)