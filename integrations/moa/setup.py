from setuptools import setup, find_packages

with open("README.md", encoding="utf-8") as f:
    long_description = f.read()

setup(
    name="moa",
    version="0.1.0",
    description=(
        "Master of Apps: safety-first, model-agnostic agent framework "
        "with hard-constraint action gating and universe-separated memory."
    ),
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="Yoder23",
    url="https://github.com/Yoder23/moa",
    license="MIT",
    packages=find_packages(exclude=["tests", "examples"]),
    python_requires=">=3.10",
    install_requires=[],          # Core is stdlib-only
    extras_require={
        "openai":    ["openai>=1.0"],
        "anthropic": ["anthropic>=0.20"],
        "ollama":    ["requests>=2.28"],
        "hf":        ["transformers>=4.35", "accelerate>=0.24"],
        "layercake": ["torch>=2.0", "sentencepiece>=0.1.99"],
        "all": [
            "openai>=1.0",
            "anthropic>=0.20",
            "requests>=2.28",
            "transformers>=4.35",
            "accelerate>=0.24",
        ],
        "dev": [
            "pytest>=7.0",
            "pytest-cov",
        ],
    },
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    keywords=[
        "agent", "llm", "ai", "safety", "ooda",
        "gpt", "claude", "ollama", "layercake",
    ],
)
