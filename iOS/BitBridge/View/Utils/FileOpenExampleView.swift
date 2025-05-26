import SwiftUI

struct FileOpenExampleView: View {
    @State private var filePath: String = ""
    @State private var showingQuickLook = false
    @State private var fileURL: URL?
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Open File from Path")
                .font(.title2)
                .fontWeight(.bold)
            
            // File path input
            TextField("Enter file path", text: $filePath)
                .textFieldStyle(RoundedBorderTextFieldStyle())
                .padding(.horizontal)
            
            if !filePath.isEmpty {
                VStack(spacing: 12) {
                    // Method 1: Quick Look Preview
                    Button("Preview with Quick Look") {
                        openWithQuickLook()
                    }
                    .buttonStyle(.borderedProminent)
                    
                    // Method 2: Open with system default app
                    Button("Open with System App") {
                        openWithSystemApp()
                    }
                    .buttonStyle(.bordered)
                    
                    // Method 3: Share file
                    Button("Share File") {
                        shareFile()
                    }
                    .buttonStyle(.bordered)
                    
                    // Method 4: Read file content (for text files)
                    Button("Read File Content") {
                        readFileContent()
                    }
                    .buttonStyle(.bordered)
                }
            }
            
            Spacer()
        }
        .padding()
        .sheet(isPresented: $showingQuickLook) {
            if let url = fileURL {
                FilePreviewView(fileURL: url)
            }
        }
    }
    
    private func openWithQuickLook() {
        guard let url = FileOpener.getFileURL(from: filePath) else {
            print("Invalid file path")
            return
        }
        
        if FileOpener.fileExists(at: url.path) {
            fileURL = url
            showingQuickLook = true
        } else {
            print("File does not exist at path: \(filePath)")
        }
    }
    
    private func openWithSystemApp() {
        guard let url = FileOpener.getFileURL(from: filePath) else {
            print("Invalid file path")
            return
        }
        
        if FileOpener.fileExists(at: url.path) {
            FileOpener.openWithSystemApp(fileURL: url)
        } else {
            print("File does not exist at path: \(filePath)")
        }
    }
    
    private func shareFile() {
        guard let url = FileOpener.getFileURL(from: filePath) else {
            print("Invalid file path")
            return
        }
        
        if FileOpener.fileExists(at: url.path) {
            FileOpener.shareFile(fileURL: url)
        } else {
            print("File does not exist at path: \(filePath)")
        }
    }
    
    private func readFileContent() {
        guard let url = FileOpener.getFileURL(from: filePath) else {
            print("Invalid file path")
            return
        }
        
        if let content = FileOpener.readFileContent(fileURL: url) {
            print("File content: \(content)")
        }
    }
}

#Preview {
    FileOpenExampleView()
} 