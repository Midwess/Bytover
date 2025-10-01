import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const compat = new FlatCompat({
  baseDirectory: __dirname,
});

const eslintConfig = [
  ...compat.extends("next/core-web-vitals", "next/typescript"),
  {
    rules: {
      "no-console": "off", // allow console.log
      "react-hooks/rules-of-hooks": "off", // allow useState/useEffect anywhere (not recommended in production)
      "react-hooks/exhaustive-deps": "off", // optional: disables checking dependency arrays
    },
  },
];

export default eslintConfig;
