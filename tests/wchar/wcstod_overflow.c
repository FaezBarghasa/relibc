#include <wchar.h>
#include <errno.h>
#include <stdio.h>
#include <math.h>
#include <assert.h>

int main() {
    errno = 0;
    wchar_t *end;
    // 1e309 is larger than DBL_MAX (approx 1.8e308)
    double d = wcstod(L"1e309", &end);
    if (isinf(d) && errno == ERANGE) {
        printf("Success: Overflow detected\n");
    } else {
        printf("Failure: d=%f, errno=%d\n", d, errno);
        return 1;
    }
    return 0;
}
