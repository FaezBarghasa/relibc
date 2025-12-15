#ifndef _WCTYPE_H
#define _WCTYPE_H

#include <wchar.h>

#ifdef __cplusplus
extern "C" {
#endif

int iswalnum(wint_t wc);
int iswalpha(wint_t wc);
int iswcntrl(wint_t wc);
int iswdigit(wint_t wc);
int iswgraph(wint_t wc);
int iswlower(wint_t wc);
int iswprint(wint_t wc);
int iswpunct(wint_t wc);
int iswspace(wint_t wc);
int iswupper(wint_t wc);
int iswxdigit(wint_t wc);
int iswblank(wint_t wc);

wint_t towlower(wint_t wc);
wint_t towupper(wint_t wc);

#ifdef __cplusplus
}
#endif

#endif /* _WCTYPE_H */
