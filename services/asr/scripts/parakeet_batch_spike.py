"""Compatibility shim for the ASR core Parakeet batch spike."""

from openwhisper_asr.engines.parakeet_batch import main

if __name__ == "__main__":
    raise SystemExit(main())
