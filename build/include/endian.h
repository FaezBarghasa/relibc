#ifndef _ENDIAN_H
#define _ENDIAN_H

#include <machine/endian.h>
#include <stdint.h>
#include <netinet/in.h>

#define htobe16(x) htons(x)
#define htole16(x) (x)
#define be16toh(x) ntohs(x)
#define le16toh(x) (x)

#define htobe32(x) htonl(x)
#define htole32(x) (x)
#define be32toh(x) ntohl(x)
#define le32toh(x) (x)

// TODO: Optimized 64-bit swap
#define htobe64(x) (((uint64_t)htonl((x) & 0xFFFFFFFF) << 32) | htonl((x) >> 32))
#define htole64(x) (x)
#define be64toh(x) (((uint64_t)ntohl((x) & 0xFFFFFFFF) << 32) | ntohl((x) >> 32))
#define le64toh(x) (x)

#endif /* _ENDIAN_H */
