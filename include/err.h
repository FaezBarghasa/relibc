#ifndef _ERR_H
#define _ERR_H

#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

void err(int eval, const char *fmt, ...);
void errx(int eval, const char *fmt, ...);
void warn(const char *fmt, ...);
void warnx(const char *fmt, ...);
void verr(int eval, const char *fmt, va_list args);
void verrx(int eval, const char *fmt, va_list args);
void vwarn(const char *fmt, va_list args);
void vwarnx(const char *fmt, va_list args);

void errc(int eval, int code, const char *fmt, ...);
void warnc(int code, const char *fmt, ...);
void verrc(int eval, int code, const char *fmt, va_list args);
void vwarnc(int code, const char *fmt, va_list args);
void err_set_file(void *fp);
void err_set_exit(void (*ef)(int));

#ifdef __cplusplus
}
#endif

#endif /* _ERR_H */
