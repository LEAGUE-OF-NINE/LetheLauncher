#!/bin/bash

# LetheLauncher Build Script
echo "Building LetheLauncher..."

# Restore dependencies
echo "Restoring dependencies..."
dotnet restore LetheLauncher.sln

# Build in Release configuration
echo "Building in Release configuration..."
dotnet build LetheLauncher.sln --configuration Release --no-restore

# Publish the application
echo "Publishing application..."
dotnet publish LetheLauncher/LetheLauncher.csproj --configuration Release --runtime win-x64 --self-contained true --output ./bin/publish

echo "Build complete!"
echo "Distribution package created in: bin/publish/"
echo "To create the final distribution package, run: dotnet publish --configuration Release"
