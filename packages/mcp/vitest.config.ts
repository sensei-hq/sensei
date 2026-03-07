import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";

export default mergeConfig(base, {
  test: { passWithNoTests: true },
});
