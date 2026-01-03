#include <assert.h>
#include <fenv.h>
#include <stdio.h>

#include "test_helpers.h"

int main(void) {
  fenv_t env;
  int res;

  printf("Testing fenv...\n");

  // Clear all exceptions
  feclearexcept(FE_ALL_EXCEPT);
  assert(fetestexcept(FE_ALL_EXCEPT) == 0);

  // Raise an exception
  feraiseexcept(FE_DIVBYZERO);
  assert(fetestexcept(FE_DIVBYZERO) != 0);

  // Get and set rounding mode
  res = fesetround(FE_UPWARD);
  assert(res == 0);
  assert(fegetround() == FE_UPWARD);

  res = fesetround(FE_DOWNWARD);
  assert(res == 0);
  assert(fegetround() == FE_DOWNWARD);

  // Save and restore environment
  fegetenv(&env);
  fesetround(FE_TOWARDZERO);
  assert(fegetround() == FE_TOWARDZERO);
  fesetenv(&env);
  assert(fegetround() == FE_DOWNWARD);

  printf("fenv tests passed!\n");
  return 0;
}
