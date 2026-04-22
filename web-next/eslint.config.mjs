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
      "react-hooks/refs": "off", // animate-ui passes refs to floating-ui / cloneElement during render
      "react-hooks/static-components": "off", // animate-ui uses motion.create() in render
      "react-hooks/immutability": "off", // animate-ui's useDataState returns a ref captured in its own callback
      "react-hooks/preserve-manual-memoization": "off", // animate-ui's useCallback deps are managed by hand
      "@next/next/no-img-element": "off", // allow raw <img> for CDN-hosted assets and mockup images
    },
  },
];

export default eslintConfig;
