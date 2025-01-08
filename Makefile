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

default:
	cargo rustc --release --bin monty --features=embed -- -C target-cpu=native --emit link=$(NAME)
	
raw:
	cargo rustc --release --bin monty --features=embed,raw -- -C target-cpu=native --emit link=$(NAME)

montytest:
	cargo +stable rustc --release --bin monty --features=uci-minimal,tunable -- -C target-cpu=native --emit link=$(NAME)

noembed:
	cargo rustc --release --bin monty -- -C target-cpu=native --emit link=$(NAME)

gen:
	cargo rustc --release --package datagen --bin datagen -- -C target-cpu=native --emit link=$(NAME)