//
//  PeerSharingProgress.swift
//  BitBridge
//
//  Created by Tien Dang on 4/25/25.
//

import SwiftUI
import Foundation

struct CircularProgressView: View {
    let progress: Double

    var body: some View {
        ZStack {
            Group {
                RoundedRectangle(cornerRadius: 10)
                    .scale(0.4)
                    .foregroundColor(Theme.BluePrimary.color)
            }
            Circle()
                .stroke(
                    Theme.Gray.color.opacity(0.15),
                    lineWidth: 4
                )
            Circle()
                .trim(from: 0, to: progress)
                .stroke(
                    Theme.BluePrimary.color,
                    style: StrokeStyle(
                        lineWidth: 4,
                        lineCap: .round
                    )
                )
                .rotationEffect(.degrees(-90))
                .animation(.easeOut(duration: 1), value: progress)

        }
    }
}

struct SharingProgress: View {
    let progress: Double
    var body: some View {
        CircularProgressView(progress: self.progress)
    }
}

#Preview {
    SharingProgress(progress: 0.6)
        .padding()
        .frame(width: 300, height: 300)
}
