//
//  SettingView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import Metal

struct SettingView: View {
    @State private var time: Double = 0
    @State private var startTime = Date.now

    var body: some View {
        ZStack {
            Theme.darkBgGradient
            Button(action: {}) {
                ZStack {
                    TimelineView(.animation) { timeline in
                        let elapsedTime = startTime.distance(to: timeline.date)
                        ZStack {
                            Circle()
                                .fill(Theme.LightViolet.color.opacity(0.9))
                                .visualEffect { content, proxy in
                                    content
                                        .colorEffect(
                                            ShaderLibrary.circleWave(
                                                .float2(proxy.size),
                                                .float(elapsedTime * 3)
                                            )
                                        )
                                }
                        }
                    }
                    .padding(.all, 25)
                    Circle()
                        .fill(Theme.circlePrimaryGradient)
                        .stroke(Theme.PrimaryViolet.color, lineWidth: 3)
                        .frame(width: 90)
                    Text("Share")
                        .foregroundStyle(Theme.LightViolet.color)
                        .modifier(Label1())
                }
            }
            .frame(width: 320, height: 320)
        }
        .ignoresSafeArea()
    }
}

#Preview {
    SettingView()
}
