#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

int main() {
  mbstate_t state;
  memset(&state, 0, sizeof state);

  // Banana: \xF0\x9F\x8D\x8C (4 bytes)
  char part1[] = "\xF0\x9F"; // First 2 bytes
  char part2[] = "\x8D\x8C"; // Last 2 bytes
  wchar_t wc = 0;

  size_t rc = mbrtowc(&wc, part1, 2, &state);
  if (rc != (size_t)-2) {
    printf("Fail 1: Expected -2 (incomplete), got %zu\n", rc);
    return 1;
  }

  rc = mbrtowc(&wc, part2, 2, &state);
  if (rc != 2) { // Should consume 2 more bytes
    printf("Fail 2: Expected 2 (bytes consumed), got %zu\n", rc);
    return 1;
  }

  if (wc != 0x1F34C) {
    printf("Fail 3: Expected 0x1F34C, got %x\n", wc);
    return 1;
  }

  printf("mbrtowc restart success\n");
  return 0;
}
