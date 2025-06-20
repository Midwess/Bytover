//
//  InternetImage.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 30/3/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct Avartar: View {
    let avatar: AvatarViewModel

    @State var uiImage: UIImage?

    private func backgroundColor() -> Color {
        // Check if any of the dominant color values are nil
        guard let r = avatar.dominant_color_r,
              let g = avatar.dominant_color_g,
              let b = avatar.dominant_color_b else {
            return .gray
        }

        // If all values are available, create color from RGB values
        return Color(red: Double(r) / 255.0,
                     green: Double(g) / 255.0,
                     blue: Double(b) / 255.0)
    }

    var body: some View {
        VStack {
            if let uiImage = uiImage {
                Image(uiImage: uiImage)
                    .resizable()
                    .scaledToFit()
                    .padding(.all, 5)
                    .background(Circle()
                        .fill(backgroundColor().opacity(0.7)))
            } else {
                Circle().fill(.gray)
            }
        }
        .task {
            do {
                let (data, _) = try await URLSession.shared.data(from: URL(string: avatar.url)!)
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
    Avartar(avatar: AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Bear.png?r=146&g=108&b=85", dominant_color_r: 146, dominant_color_g: 108, dominant_color_b: 85))
}
