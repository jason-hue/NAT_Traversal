# Claude Code Review Configuration

This repository is configured to use Claude Code Review for automated code review on pull requests.

## Setup Instructions

### 1. Install GitHub App
1. Go to your repository settings
2. Click on "Install GitHub App"
3. Search for "Claude Code Review" and install it
4. Grant necessary permissions (read code, write pull requests)

### 2. Configure API Key
1. Go to repository Settings → Secrets and variables → Actions
2. Add a new repository secret named `ANTHROPIC_API_KEY`
3. Use your Anthropic API key as the value

### 3. Verify Configuration
The workflow will automatically run on new pull requests and check:
- Code formatting
- Clippy lints
- Tests
- Claude Code Review

## Workflow Files

- `.github/workflows/claude-review.yml` - Main Claude Code Review workflow
- `.github/workflows/ci.yml` - Basic CI pipeline
- `.github/slash-commands/` - Directory for slash commands

## Slash Commands

Use these in pull request comments:
- `/review` - Trigger immediate review
- `/explain` - Explain code concepts
- `/suggest` - Get improvement suggestions
- `/docs` - Documentation help

## Troubleshooting

### Common Issues

1. **Invalid API Key Error**
   - Verify ANTHROPIC_API_KEY secret is correctly set
   - Ensure the API key has active credits
   - Check the key is not expired

2. **Slash Commands Directory Not Found**
   - Ensure `.github/slash-commands/` directory exists
   - The directory should contain at least a README.md file

3. **Permission Errors**
   - Verify GitHub App has necessary permissions
   - Check that the app is installed on the repository

### Debugging

Check the workflow logs for detailed error information:
1. Go to Actions tab
2. Click on the failed workflow run
3. Review the logs for each step