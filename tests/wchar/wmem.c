#include <assert.h>
#include <stdio.h>
#include <wchar.h>

int main() {
  wchar_t buf[10];
  wmemset(buf, L'A', 5);
  wmemset(buf + 5, L'B', 5);

  assert(buf[0] == L'A');
  assert(buf[4] == L'A');
  assert(buf[5] == L'B');
  assert(buf[9] == L'B');

  assert(wmemchr(buf, L'B', 10) == buf + 5);
  assert(wmemchr(buf, L'C', 10) == NULL);

  wchar_t buf2[10];
  wmemset(buf2, L'A', 5);
  wmemset(buf2 + 5, L'B', 5);
  assert(wmemcmp(buf, buf2, 10) == 0);

  buf2[9] = L'C';
  assert(wmemcmp(buf, buf2, 10) < 0);

  printf("wmem tests passed\n");
  return 0;
}
