//
//  InternetImage.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 30/3/25.
//

import SwiftUI
import Foundation

struct Avartar: View {
    let url: URL
    
    @State var uiImage: UIImage?
    
    var body: some View {
        VStack {
            if let uiImage = uiImage {
                Image(uiImage: uiImage)
                    .resizable()
                    .scaledToFit()
                    .padding(.all, 5)
                    .background(Circle()
                        .fill(Color(uiImage.backgroundColor)))
            }
            else {
                Circle().fill(.gray)
            }
        }
        .task {
            do {
                let (data, _) = try await URLSession.shared.data(from: url)
                if let downloadedImage = UIImage(data: data) {
                    uiImage = downloadedImage
                }
            } catch {
                print("Error loading image: \(error)")
            }
        }
    }
}

#Preview {
    Avartar(url: URL(string: "https://cdn.devlog.studio/public/animal_avatars/Cat.png")!)
}
