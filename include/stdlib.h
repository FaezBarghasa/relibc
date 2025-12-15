#ifndef _STDLIB_H
#define _STDLIB_H

#include <bits/stdlib.h>
#include <stddef.h>

#define EXIT_FAILURE 1
#define EXIT_SUCCESS 0

#ifdef __cplusplus
extern "C" {
#endif

void *malloc(size_t size);
void free(void *ptr);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void *aligned_alloc(size_t alignment, size_t size);
void *reallocarray(void *ptr, size_t nmemb, size_t size);
int posix_memalign(void **memptr, size_t alignment, size_t size);

void exit(int status);
void abort(void);
int atexit(void (*func)(void));

int atoi(const char *nptr);
long atol(const char *nptr);
long long atoll(const char *nptr);
double atof(const char *nptr);

long strtol(const char *nptr, char **endptr, int base);
long long strtoll(const char *nptr, char **endptr, int base);
unsigned long strtoul(const char *nptr, char **endptr, int base);
unsigned long long strtoull(const char *nptr, char **endptr, int base);

float strtof(const char *nptr, char **endptr);
double strtod(const char *nptr, char **endptr);
long double strtold(const char *nptr, char **endptr);

void _Exit(int status);

char *getenv(const char *name);
int setenv(const char *name, const char *value, int overwrite);
int unsetenv(const char *name);
char *mkdtemp(char *template);

int system(const char *command);

void *bsearch(const void *key, const void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));

int abs(int j);
long labs(long j);
long long llabs(long long j);

typedef struct { int quot; int rem; } div_t;
typedef struct { long quot; long rem; } ldiv_t;
typedef struct { long long quot; long long rem; } lldiv_t;

div_t div(int numer, int denom);
ldiv_t ldiv(long numer, long denom);
lldiv_t lldiv(long long numer, long long denom);

int rand(void);
int rand_r(unsigned int *seedp);
void srand(unsigned int seed);

long a64l(const char *str64);
char *l64a(long value);

int mkstemp(char *template);
int mkostemp(char *template, int flags);
int mkostemps(char *template, int suffixlen, int flags);
char *mktemp(char *template);

char *realpath(const char *path, char *resolved_path);

char *ptsname(int fd);
int unlockpt(int fd);
int grantpt(int fd);
int posix_openpt(int flags);

long random(void);
void srandom(unsigned int seed);
char *initstate(unsigned int seed, char *statebuf, size_t statelen);
char *setstate(char *statebuf);

double drand48(void);
double erand48(unsigned short xsubi[3]);
long lrand48(void);
long nrand48(unsigned short xsubi[3]);
long mrand48(void);
long jrand48(unsigned short xsubi[3]);
void srand48(long seedval);
unsigned short *seed48(unsigned short seed16v[3]);
void lcong48(unsigned short param[7]);

int putenv(char *string);

#ifdef __cplusplus
}
#endif

#endif /* _STDLIB_H */
