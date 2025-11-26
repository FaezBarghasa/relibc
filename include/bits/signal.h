#ifndef _BITS_SIGNAL_H
#define _BITS_SIGNAL_H

#include <sys/types.h>

#define SIG_DFL ((void (*)(int))0)
#define SIG_IGN ((void (*)(int))1)
#define SIG_ERR ((void (*)(int))-1)
#define SIG_HOLD ((void (*)(int))2)

#define SIG_BLOCK 0
#define SIG_UNBLOCK 1
#define SIG_SETMASK 2

#define SIGHUP    1
#define SIGINT    2
#define SIGQUIT   3
#define SIGILL    4
#define SIGTRAP   5
#define SIGABRT   6
#define SIGIOT    6
#define SIGBUS    7
#define SIGFPE    8
#define SIGKILL   9
#define SIGUSR1   10
#define SIGSEGV   11
#define SIGUSR2   12
#define SIGPIPE   13
#define SIGALRM   14
#define SIGTERM   15
#define SIGSTKFLT 16
#define SIGCHLD   17
#define SIGCONT   18
#define SIGSTOP   19
#define SIGTSTP   20
#define SIGTTIN   21
#define SIGTTOU   22
#define SIGURG    23
#define SIGXCPU   24
#define SIGXFSZ   25
#define SIGVTALRM 26
#define SIGPROF   27
#define SIGWINCH  28
#define SIGIO     29
#define SIGPOLL   SIGIO
#define SIGPWR    30
#define SIGSYS    31
#define SIGUNUSED SIGSYS



#define SA_NOCLDSTOP 1
#define SA_NOCLDWAIT 2
#define SA_SIGINFO   4
#define SA_ONSTACK   0x08000000
#define SA_RESTART   0x10000000
#define SA_NODEFER   0x40000000
#define SA_RESETHAND 0x80000000
#define SA_RESTORER  0x04000000

#define SI_USER 0
#define SI_QUEUE -1
#define SI_TIMER 1
#define SI_ASYNCIO 2
#define SI_MESGQ 3

#define SS_ONSTACK 1
#define SS_DISABLE 2
#define MINSIGSTKSZ 2048
#define SIGSTKSZ 8096

typedef struct siginfo siginfo_t;
typedef unsigned long long sigset_t;
typedef struct ucontext ucontext_t;

typedef struct {
    long gregs[23];
    void *fpregs;
    unsigned long __private[8];
} mcontext_t;

struct sigaltstack {
    void *ss_sp;
    int ss_flags;
    size_t ss_size;
};

typedef struct sigaltstack stack_t;

struct ucontext {
    unsigned long uc_flags;
    struct ucontext *uc_link;
    stack_t uc_stack;
    mcontext_t uc_mcontext;
    sigset_t uc_sigmask;
    unsigned char __private[512];
};

union sigval {
    int sival_int;
    void *sival_ptr;
};

struct siginfo {
    int si_signo;
    int si_errno;
    int si_code;
    pid_t si_pid;
    uid_t si_uid;
    void *si_addr;
    int si_status;
    union sigval si_value;
};



struct sigaction {
  union {
    void (*sa_handler)(int);
    void (*sa_sigaction)(int, siginfo_t *, void *);
  };
  unsigned long sa_flags;
  void (*sa_restorer)(void);
  sigset_t sa_mask;
};

int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);
int raise(int sig);
int sigemptyset(sigset_t *set);
int sigaddset(sigset_t *set, int signum);
int sigdelset(sigset_t *set, int signum);
int sigismember(const sigset_t *set, int signum);
int sigfillset(sigset_t *set);
int sigaltstack(const stack_t *ss, stack_t *old_ss);
void (*signal(int sig, void (*func)(int)))(int);

#endif // _BITS_SIGNAL_H
