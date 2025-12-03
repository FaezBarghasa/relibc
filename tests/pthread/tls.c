#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <pthread.h>

#include "common.h"

__thread int tls_var = 123;

void *thread_main(void *arg) {
    assert(tls_var == 123);
    tls_var = 456;
    assert(tls_var == 456);
    return NULL;
}

int main(void) {
    int status;

    assert(tls_var == 123);

    pthread_t thread;
    if ((status = pthread_create(&thread, NULL, thread_main, NULL)) != 0) {
        return fail(status, "create thread");
    }

    void *retval;
    if ((status = pthread_join(thread, &retval)) != 0) {
        return fail(status, "join thread");
    }

    assert(tls_var == 123);

    return EXIT_SUCCESS;
}
