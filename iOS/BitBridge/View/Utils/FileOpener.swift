import SwiftUI
import QuickLook
import UIKit

// MARK: - Quick Look Preview
struct FilePreviewView: UIViewControllerRepresentable {
    let fileURL: URL
    
    func makeUIViewController(context: Context) -> QLPreviewController {
        let controller = QLPreviewController()
        controller.dataSource = context.coordinator
        return controller
    }
    
    func updateUIViewController(_ uiViewController: QLPreviewController, context: Context) {}
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    class Coordinator: NSObject, QLPreviewControllerDataSource {
        let parent: FilePreviewView
        
        init(_ parent: FilePreviewView) {
            self.parent = parent
        }
        
        func numberOfPreviewItems(in controller: QLPreviewController) -> Int {
            return 1
        }
        
        func previewController(_ controller: QLPreviewController, previewItemAt index: Int) -> QLPreviewItem {
            return parent.fileURL as QLPreviewItem
        }
    }
}

// MARK: - File Opening Utilities
class FileOpener: ObservableObject {
    
    // Method 1: Open with Quick Look Preview
    static func openWithQuickLook(fileURL: URL) -> some View {
        FilePreviewView(fileURL: fileURL)
    }
    
    // Method 2: Open with system default app using UIDocumentInteractionController
    static func openWithSystemApp(fileURL: URL, from viewController: UIViewController? = nil) {
        let documentController = UIDocumentInteractionController(url: fileURL)
        
        if let vc = viewController ?? UIApplication.shared.windows.first?.rootViewController {
            documentController.presentOpenInMenu(from: CGRect.zero, in: vc.view, animated: true)
        }
    }
    
    // Method 3: Share file using UIActivityViewController
    static func shareFile(fileURL: URL, from viewController: UIViewController? = nil) {
        let activityViewController = UIActivityViewController(activityItems: [fileURL], applicationActivities: nil)
        
        if let vc = viewController ?? UIApplication.shared.windows.first?.rootViewController {
            // For iPad
            if let popover = activityViewController.popoverPresentationController {
                popover.sourceView = vc.view
                popover.sourceRect = CGRect(x: vc.view.bounds.midX, y: vc.view.bounds.midY, width: 0, height: 0)
                popover.permittedArrowDirections = []
            }
            
            vc.present(activityViewController, animated: true)
        }
    }
    
    // Method 4: Open file with specific app using URL scheme
    static func openWithSpecificApp(fileURL: URL, appScheme: String) {
        if let encodedPath = fileURL.absoluteString.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
           let url = URL(string: "\(appScheme)://\(encodedPath)") {
            if UIApplication.shared.canOpenURL(url) {
                UIApplication.shared.open(url)
            }
        }
    }
    
    // Method 5: Read file content directly (for text files, images, etc.)
    static func readFileContent(fileURL: URL) -> String? {
        do {
            let content = try String(contentsOf: fileURL, encoding: .utf8)
            return content
        } catch {
            print("Error reading file: \(error)")
            return nil
        }
    }
    
    // Method 6: Check if file exists at path
    static func fileExists(at path: String) -> Bool {
        return FileManager.default.fileExists(atPath: path)
    }
    
    // Method 7: Get file URL from path
    static func getFileURL(from path: String) -> URL? {
        if path.hasPrefix("file://") {
            return URL(string: path)
        } else {
            return URL(fileURLWithPath: path)
        }
    }
}

// MARK: - SwiftUI View Extensions
extension View {
    func openFile(at path: String, method: FileOpenMethod = .quickLook) {
        guard let fileURL = FileOpener.getFileURL(from: path) else {
            print("Invalid file path: \(path)")
            return
        }
        
        switch method {
        case .quickLook:
            // This would need to be handled in a sheet or fullScreenCover
            break
        case .systemApp:
            FileOpener.openWithSystemApp(fileURL: fileURL)
        case .share:
            FileOpener.shareFile(fileURL: fileURL)
        case .specificApp(let scheme):
            FileOpener.openWithSpecificApp(fileURL: fileURL, appScheme: scheme)
        }
    }
}

enum FileOpenMethod {
    case quickLook
    case systemApp
    case share
    case specificApp(scheme: String)
} 