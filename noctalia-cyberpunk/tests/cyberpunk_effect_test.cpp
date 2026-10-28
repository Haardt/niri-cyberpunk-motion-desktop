#include "render/core/cyberpunk_effect.h"
#include "render/core/cyberpunk_scan.h"
#include "test_check.h"

int main() {
  // Every strip travels downward without jumps, then fully leaves the element.
  for (float seed : {0.0F, 1.0F, 71.0F, 1234.0F, 65520.0F}) {
    TEST_CHECK(cyberpunk::scanDuration(seed) >= 1600 && cyberpunk::scanDuration(seed) <= 2200);
    for (int strip = 0; strip < cyberpunk::scanCount(seed); ++strip) {
      TEST_CHECK(cyberpunk::scanStripTop(0, strip, seed) < 0);
      TEST_CHECK(cyberpunk::scanStripTop(1, strip, seed) >= 0.999F);
      float previousY = cyberpunk::scanStripTop(0, strip, seed);
      float previousOffset = 0;
      for (int frame = 1; frame <= 108; ++frame) {
        const float phase = static_cast<float>(frame) / 108.0F;
        const float y = cyberpunk::scanStripTop(phase, strip, seed);
        const float offset = cyberpunk::scanOffset(phase, strip, seed);
        TEST_CHECK(y >= previousY);
        TEST_CHECK(std::abs(offset) <= 10.0F);
        TEST_CHECK(std::abs(offset - previousOffset) < 0.6F);
        previousY = y;
        previousOffset = offset;
      }
      TEST_CHECK(cyberpunk::scanOffset(1, strip, seed) == 0);
    }
  }
  TEST_CHECK(cyberpunk::scanWidth(1) != cyberpunk::scanWidth(71));
  TEST_CHECK(cyberpunk::scanDuration(1) != cyberpunk::scanDuration(71));
  TEST_CHECK(cyberpunk::progress(-20.0F) == 0.0F);
  TEST_CHECK(cyberpunk::envelope(0.0F) == 0.0F);
  TEST_CHECK(cyberpunk::envelope(0.5F) > 0.99F);
  // A delayed frame must settle the effect rather than restart or loop it.
  TEST_CHECK(cyberpunk::progress(10000.0F) == 1.0F);
  TEST_CHECK(cyberpunk::envelope(cyberpunk::progress(10000.0F)) == 0.0F);
  for (int ms = 0; ms < 1000; ++ms) {
    const float amount = cyberpunk::envelope(cyberpunk::progress(static_cast<float>(ms)));
    TEST_CHECK(amount >= 0.0F && amount <= 1.0F);
    if (ms >= 320)
      TEST_CHECK(amount == 0.0F);
  }
}
