#ifndef _WCHAR_H
#define _WCHAR_H

#include <bits/wchar.h>
#include <stdio.h>
#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int __count;
    union {
        unsigned int __wch;
        char __wchb[4];
    } __value;
} mbstate_t;

wint_t btowc(int c);
wint_t fgetwc(FILE *stream);
wchar_t *fgetws(wchar_t *ws, int n, FILE *stream);
wint_t fputwc(wchar_t wc, FILE *stream);
int fputws(const wchar_t *ws, FILE *stream);
int fwide(FILE *stream, int mode);
wint_t getwc(FILE *stream);
wint_t getwchar(void);
int mbsinit(const mbstate_t *ps);
size_t mbrlen(const char *s, size_t n, mbstate_t *ps);
size_t mbrtowc(wchar_t *pwc, const char *s, size_t n, mbstate_t *ps);
size_t mbsrtowcs(wchar_t *dst, const char **src, size_t len, mbstate_t *ps);
size_t mbsnrtowcs(wchar_t *dst, const char **src, size_t nwc, size_t len, mbstate_t *ps);
wint_t putwc(wchar_t wc, FILE *stream);
wint_t putwchar(wchar_t wc);
wint_t ungetwc(wint_t wc, FILE *stream);
size_t wcrtomb(char *s, wchar_t wc, mbstate_t *ps);
wchar_t *wcscat(wchar_t *ws1, const wchar_t *ws2);
wchar_t *wcschr(const wchar_t *ws, wchar_t wc);
int wcscmp(const wchar_t *ws1, const wchar_t *ws2);
int wcscoll(const wchar_t *ws1, const wchar_t *ws2);
wchar_t *wcscpy(wchar_t *ws1, const wchar_t *ws2);
size_t wcscspn(const wchar_t *wcs, const wchar_t *set);
size_t wcslen(const wchar_t *ws);
wchar_t *wcsncat(wchar_t *ws1, const wchar_t *ws2, size_t n);
int wcsncmp(const wchar_t *ws1, const wchar_t *ws2, size_t n);
wchar_t *wcsncpy(wchar_t *ws1, const wchar_t *ws2, size_t n);
wchar_t *wcspbrk(const wchar_t *wcs, const wchar_t *set);
wchar_t *wcsrchr(const wchar_t *ws1, wchar_t wc);
size_t wcsspn(const wchar_t *wcs, const wchar_t *set);
wchar_t *wcsstr(const wchar_t *ws1, const wchar_t *ws2);
double wcstod(const wchar_t *ptr, wchar_t **end);
wchar_t *wcstok(wchar_t *wcs, const wchar_t *delim, wchar_t **state);
size_t wcsftime(wchar_t *wcs, size_t maxsize, const wchar_t *format, const struct tm *timptr);
wchar_t *wcpcpy(wchar_t *d, const wchar_t *s);
wchar_t *wcpncpy(wchar_t *d, const wchar_t *s, size_t n);
wchar_t *wcsdup(const wchar_t *s);
size_t wcsnlen(const wchar_t *s, size_t maxlen);
size_t wcsnrtombs(char *dest, const wchar_t **src, size_t nwc, size_t len, mbstate_t *ps);
size_t wcsrtombs(char *s, const wchar_t **ws, size_t n, mbstate_t *st);

int wprintf(const wchar_t *format, ...);
int fwprintf(FILE *stream, const wchar_t *format, ...);
int swprintf(wchar_t *s, size_t n, const wchar_t *format, ...);
int vwprintf(const wchar_t *format, va_list arg);
int vfwprintf(FILE *stream, const wchar_t *format, va_list arg);
int vswprintf(wchar_t *s, size_t n, const wchar_t *format, va_list arg);

int wscanf(const wchar_t *format, ...);
int fwscanf(FILE *stream, const wchar_t *format, ...);
int swscanf(const wchar_t *s, const wchar_t *format, ...);
int vwscanf(const wchar_t *format, va_list arg);
int vfwscanf(FILE *stream, const wchar_t *format, va_list arg);
int vswscanf(const wchar_t *s, const wchar_t *format, va_list arg);

long wcstol(const wchar_t *nptr, wchar_t **endptr, int base);
long long wcstoll(const wchar_t *nptr, wchar_t **endptr, int base);
unsigned long wcstoul(const wchar_t *nptr, wchar_t **endptr, int base);
unsigned long long wcstoull(const wchar_t *nptr, wchar_t **endptr, int base);

#ifdef __cplusplus
}
#endif

#endif /* _WCHAR_H */
