#ifndef _BITS_STDIO_H
#define _BITS_STDIO_H

// XXX: this is only here because cbindgen can't handle string constants
#define P_tmpdir "/tmp"

#define BUFSIZ 1024
#define FILENAME_MAX 4096
#define L_tmpnam 7
#define TMP_MAX 2147483647
#define _IOFBF 0
#define _IOLBF 1
#define _IONBF 2
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

typedef struct FILE FILE;

// A typedef doesn't suffice, because libgmp uses this definition to check if
// STDIO was loaded.
#define FILE FILE

#endif /* _BITS_STDIO_H */
