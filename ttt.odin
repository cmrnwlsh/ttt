package main

import "core:os"
import "core:fmt"
import "core:strings"
import sys "core:sys/posix"
import vt "core:terminal/ansi"

STDIN  :: sys.FD(sys.STDIN_FILENO)

Style  :: struct { color: string, glyph: byte }
PLAYER :: Style { vt.BOLD + ";" + vt.FG_BRIGHT_BLUE, 'X' }
COMP   :: Style { vt.BOLD + ";" + vt.FG_RED, 'O' }
EMPTY  :: '.'

Dir :: enum { Up, Down, Left, Right }
DIRS :: [Dir][2]i8 {
	.Up    = { 0, -1 },
	.Down  = { 0, +1 },
	.Left  = { -1, 0 },
	.Right = { +1, 0 },
}

sel : [2]i8
grid := [3][3]byte{0..=2 = {0..=2 = '.'}}

cell :: proc(pos: [2]i8) -> ^byte {
	return &grid[pos.y][pos.x]
}

rare :: proc() -> (term: sys.termios) {
	sys.tcgetattr(STDIN, &term)

	r := term
	r.c_lflag -= {.ICANON, .ECHO, .ISIG}
	r.c_cc[.VMIN]  = 1
	r.c_cc[.VTIME] = 0

	sys.tcsetattr(STDIN, .TCSAFLUSH, &r)
	fmt.print(vt.CSI + vt.DECTCEM_HIDE)

	return
}

restore :: proc(term: ^sys.termios) {
	sys.tcsetattr(STDIN, .TCSAFLUSH, term)
	fmt.print(vt.CSI + vt.DECTCEM_SHOW)
}

in_range :: proc(pos: [2]i8) -> bool {
	return pos[0] >= 0 && pos[0] <=2 && pos[1] >= 0 && pos[1] <= 2
}

input :: proc(ch: byte) {
	tmp := sel
	switch ch {
	case 'w':
		tmp += DIRS[.Up]
	case 'a':
		tmp += DIRS[.Left]
	case 's':
		tmp += DIRS[.Down]
	case 'd':
		tmp += DIRS[.Right]
	case ' ':
		if t := cell(tmp); t^ == EMPTY {
			t^ = PLAYER.glyph
		}
	}
	if in_range(tmp) do sel = tmp
}

draw :: proc(sb: ^strings.Builder) {
	write_string, write_byte, write_rune, to_string, reset ::
	strings.write_string, strings.write_byte, strings.write_rune,
	strings.to_string, strings.builder_reset

	write_cell :: proc(sb: ^strings.Builder, glyph: byte, selected: bool) {
		write_string(sb, vt.CSI + vt.RESET)
		switch glyph {
		case PLAYER.glyph: write_string(sb, ";" + PLAYER.color)
		case COMP.glyph:   write_string(sb, ";" + COMP.color)
		}
		if selected do write_string(sb, ";" + vt.INVERT)
		write_string(sb, vt.SGR)
		write_byte(sb, ' ')
		write_byte(sb, glyph)
		write_byte(sb, ' ')
		write_string(sb, vt.CSI + vt.RESET + vt.SGR)
	}

	write_string(sb, "╔═══╦═══╦═══╗\n")
	for row, y in grid {
		for glyph, x in row {
			write_rune(sb, '║')
			write_cell(sb, glyph, sel == {i8(x), i8(y)})
		}
		write_string(sb, "║\n")
		if y != 2 do write_string(sb, "╠═══╬═══╬═══╣\n")
	}
	write_string(sb, "╚═══╩═══╩═══╝")
	write_string(sb, "\r" + vt.CSI + "7" + vt.CUU)

	fmt.println(to_string(sb^))
	reset(sb)

}

main :: proc() {
	term := rare()
	defer restore(&term)

	sb := strings.builder_make()
	defer strings.builder_destroy(&sb)

	buf : [8]byte
	grid[1][0] = PLAYER.glyph

	for {
		draw(&sb)
		n := os.read(os.stdin, buf[:]) or_break
		if n == 0 do break

		for ch in buf[:n] {
			if ch == 'q' || ch == '\x03' do return
			input(ch)
		}
	}
}
