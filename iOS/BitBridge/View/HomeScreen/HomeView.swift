//
//  HomeView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 18/2/25.
//

import Foundation
import SwiftUI

struct HomeView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @StateObject private var core = Core()
    
    var body: some View {
        ZStack {
            Theme.darkBgGradient
            Theme.starGradient(x: 0.5, y: 0.15)
            VStack {
                LogoView(width: 20)
                    .frame(width: .infinity, height: 100)
                Text("Secure and Fastest \n file transfer")
                    .multilineTextAlignment(.center)
                    .modifier(Label1())
                    .foregroundStyle(Theme.gradientHeading)
                    .padding(.top, 10)
                Spacer()
            }
            .padding(.top, safeAreaInsets.top)
        }
        .ignoresSafeArea()
    }
}

#Preview("HomeView") {
    HomeView()
}
